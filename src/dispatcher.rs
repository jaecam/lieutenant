use crate::{command::Exec, Argument, Command, CommandSpec, Context, Input};
use slab::Slab;
use smallvec::SmallVec;

#[derive(Debug)]
pub enum RegisterError {
    /// Overlapping commands exist: two commands
    /// have an executable node at the same point.
    OverlappingCommands,
    /// Attempted to register an executable command at the root of the command graph.
    ExecutableRoot,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeKey(usize);

/// Data structure used to dispatch commands.
pub struct CommandDispatcher<C: Context> {
    // This structure acts as the root node.
    /// Stores all nodes in the command graph.
    pub nodes: Slab<Node<C>>,
    /// Children of the root node.
    pub children: SmallVec<[NodeKey; 4]>,
    /// Vector of all commands registered to this dispatcher.
    pub commands: Vec<CommandSpec<C>>,
}

impl<C: Context> Default for CommandDispatcher<C> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            children: Default::default(),
            commands: Default::default(),
        }
    }
}

impl<C> CommandDispatcher<C>
where
    C: Context,
{
    /// Creates a new `CommandDispatcher` with no registered commands.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a command to this `CommandDispatcher`.
    pub fn register(&mut self, command: impl Command<C>) -> Result<(), RegisterError>
    where
        C: 'static,
    {
        let spec = command.build();

        let mut arguments = spec.arguments.iter().peekable();

        let mut node_key: Option<NodeKey> = None;

        'argument: while let Some(argument) = arguments.peek() {
            let children = match node_key {
                Some(key) => &self.nodes[key.0].children,
                None => &self.children,
            };

            for &child_key in children {
                let child = &self.nodes[child_key.0];

                if argument == &&child.argument {
                    arguments.next();
                    node_key = Some(child_key);
                    continue 'argument;
                }
            }
            break;
        }

        for argument in arguments {
            let child = Node::from((*argument).clone());
            let child_key = NodeKey(self.nodes.insert(child));

            if let Some(node_key) = node_key {
                let node = &mut self.nodes[node_key.0];
                node.children.push(child_key);
            } else {
                self.children.push(child_key);
            }

            node_key = Some(child_key);
        }

        if let Some(key) = node_key {
            let node = &mut self.nodes[key.0];
            node.execs.push(spec.exec);
        } else {
            // Command with zero arguments?
            return Err(RegisterError::ExecutableRoot);
        }

        self.commands.push(spec);

        Ok(())
    }

    /// Method-chaining function to register a command.
    ///
    /// # Panics
    /// Panics if overlapping commands are detected. Use `register`
    /// to handle this error.
    pub fn with(mut self, command: impl Command<C>) -> Self
    where
        C: 'static,
    {
        self.register(command).unwrap();
        self
    }

    /// Dispatches a command. Returns whether a command was executed.
    pub fn dispatch(&self, ctx: &mut C, command: &str) -> Result<C::Ok, Vec<C::Error>> {
        let mut nodes = Vec::new();
        let mut errors = Vec::new();

        for child_key in &self.children {
            nodes.push((Input::new(command), child_key.0));
        }

        while let Some((mut input, node_key)) = nodes.pop() {
            let node = &self.nodes[node_key];
            let satisfies = match &node.argument {
                Argument::Literal { values } => {
                    let parsed = input.advance_until(" ");
                    values.iter().any(|value| value == parsed)
                }
                Argument::Parser { satisfies, .. } => satisfies(ctx, &mut input),
            };

            if input.is_empty() && satisfies {
                for exec in &node.execs {
                    match exec(ctx, command) {
                        Ok(ok) => return Ok(ok),
                        Err(err) => errors.push(err),
                    }
                }
                continue;
            }

            if satisfies {
                for child_key in &node.children {
                    nodes.push((input, child_key.0));
                }
            }
        }
        Err(errors)
    }

    pub fn commands(&self) -> impl Iterator<Item = &CommandSpec<C>> {
        self.commands.iter()
    }
}

/// Node on the command graph.
pub struct Node<C: Context> {
    children: SmallVec<[NodeKey; 4]>,
    argument: Argument<C>,
    execs: Vec<Exec<C>>,
}

impl<C: Context> From<Argument<C>> for Node<C> {
    fn from(argument: Argument<C>) -> Self {
        Node {
            children: Default::default(),
            argument,
            execs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    /*use super::*;
    use bstr::B;
    use smallvec::smallvec;

    #[test]
    fn parse_into_arguments() {
        let test: Vec<(&[u8], SmallVec<[&[u8]; 4]>)> = vec![
            (
                B("test 20 \"this is a string: \\\"Hello world\\\"\""),
                smallvec![B("test"), B("20"), B("this is a string: \"Hello world\"")],
            ),
            (
                B("big inputs cost big programmers with big skills"),
                smallvec![
                    B("big"),
                    B("inputs"),
                    B("cost"),
                    B("big"),
                    B("programmers"),
                    B("with"),
                    B("big"),
                    B("skills"),
                ],
            ),
        ];

        for (input, expected) in test {
            assert_eq!(CommandDispatcher::parse_into_arguments(input), expected);
        }
    }*/
}
