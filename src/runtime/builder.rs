use super::{
    ComponentInstanceId,
    ViewNode,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeBuilderError {
    NoOpenNode,
    UnclosedNodes,
    MultipleRoots,
    MissingRoot,
}

pub struct ViewTreeBuilder {
    component_instance:
        ComponentInstanceId,

    stack:
        Vec<ViewNode>,

    roots:
        Vec<ViewNode>,
}

impl ViewTreeBuilder {
    pub fn new(
        component_instance:
        ComponentInstanceId,
    ) -> Self {
        Self {
            component_instance,
            stack: Vec::new(),
            roots: Vec::new(),
        }
    }

    pub fn component_instance(
        &self,
    ) -> ComponentInstanceId {
        self.component_instance
    }

    pub fn begin(
        &mut self,
        node: ViewNode,
    ) {
        self.stack.push(node);
    }

    pub fn leaf(
        &mut self,
        node: ViewNode,
    ) {
        self.append(node);
    }

    pub fn end(
        &mut self,
    ) -> Result<(), TreeBuilderError> {
        let node =
            self.stack
                .pop()
                .ok_or(
                    TreeBuilderError::NoOpenNode,
                )?;

        self.append(node);

        Ok(())
    }

    pub fn finish(
        self,
    ) -> Result<ViewNode, TreeBuilderError> {
        if !self.stack.is_empty() {
            return Err(
                TreeBuilderError::UnclosedNodes,
            );
        }

        match self.roots.len() {
            0 => {
                Err(
                    TreeBuilderError::MissingRoot,
                )
            }

            1 => {
                Ok(
                    self.roots
                        .into_iter()
                        .next()
                        .unwrap(),
                )
            }

            _ => {
                Err(
                    TreeBuilderError::MultipleRoots,
                )
            }
        }
    }

    fn append(
        &mut self,
        node: ViewNode,
    ) {
        if let Some(parent) =
            self.stack.last_mut()
        {
            parent.children.push(node);
        } else {
            self.roots.push(node);
        }
    }
}