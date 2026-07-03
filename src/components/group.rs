//! 見た目や独自のレイアウトを持たず、複数の子要素をまとめるGroupを定義

use crate::layout::{IntoStackChildren, StackChild};

#[derive(Default)]
pub struct Group {
    children: Vec<StackChild>,
}

impl Group {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child<C>(mut self, child: C) -> Self
    where
        C: IntoStackChildren,
    {
        self.children.extend(child.into_stack_children());

        self
    }

    pub fn children<C>(mut self, children: impl IntoIterator<Item = C>) -> Self
    where
        C: IntoStackChildren,
    {
        for child in children {
            self.children.extend(child.into_stack_children());
        }

        self
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }
}

impl IntoStackChildren for Group {
    fn into_stack_children(self) -> Vec<StackChild> {
        self.children
    }
}
