use std::collections::VecDeque;

use super::{
    ActionId,
    ComponentInstanceId,
    NodeId,
};

#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    ButtonClicked,

    TextChanged {
        value: String,
    },

    TextSubmitted {
        value: String,
    },
}

#[derive(Clone, Debug)]
pub struct RuntimeAction {
    pub component_instance:
        ComponentInstanceId,

    pub node_id:
        NodeId,

    pub action_id:
        ActionId,

    pub event:
        RuntimeEvent,
}

#[derive(Default)]
pub struct ActionQueue {
    queue:
        VecDeque<RuntimeAction>,
}

impl ActionQueue {
    pub fn push(
        &mut self,
        action: RuntimeAction,
    ) {
        self.queue.push_back(action);
    }

    pub fn poll(
        &mut self,
    ) -> Option<RuntimeAction> {
        self.queue.pop_front()
    }

    pub fn is_empty(
        &self,
    ) -> bool {
        self.queue.is_empty()
    }
}