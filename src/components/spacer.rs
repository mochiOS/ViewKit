//! Stack内の余白を押し広げるSpacerを定義

use crate::layout::{IntoStackChild, StackChild};

#[derive(Clone, Copy, Debug, Default)]
pub struct Spacer;

impl Spacer {
    pub const fn new() -> Self {
        Self
    }
}

impl IntoStackChild for Spacer {
    fn into_stack_child(self) -> StackChild {
        StackChild::spacer()
    }
}
