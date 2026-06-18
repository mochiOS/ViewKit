use std::collections::HashMap;

use crate::components::{
    ButtonInteractionState,
    ScrollState,
};

use super::NodeId;

#[derive(Default)]
pub struct RuntimeStateStore {
    button_states:
        HashMap<
            NodeId,
            ButtonInteractionState,
        >,

    scroll_states:
        HashMap<
            NodeId,
            ScrollState,
        >,
}

impl RuntimeStateStore {
    pub fn button(
        &mut self,
        id: NodeId,
    ) -> ButtonInteractionState {
        self.button_states
            .entry(id)
            .or_insert_with(
                ButtonInteractionState::new,
            )
            .clone()
    }

    pub fn scroll(
        &mut self,
        id: NodeId,
    ) -> ScrollState {
        self.scroll_states
            .entry(id)
            .or_insert_with(
                ScrollState::new,
            )
            .clone()
    }

    pub fn retain_nodes(
        &mut self,
        active_nodes:
        &std::collections::HashSet<NodeId>,
    ) {
        self.button_states.retain(
            |id, _| {
                active_nodes.contains(id)
            },
        );

        self.scroll_states.retain(
            |id, _| {
                active_nodes.contains(id)
            },
        );
    }
}