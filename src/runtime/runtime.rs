use std::collections::HashSet;

use crate::view::View;

use super::{
    ActionQueue,
    ComponentInstanceId,
    RuntimeAction,
    RuntimeEvent,
    RuntimeStateStore,
    ViewAdapter,
    ViewNode,
    ViewNodeKind,
};

pub struct ViewRuntime {
    component_instance:
        ComponentInstanceId,

    tree:
        Option<ViewNode>,

    states:
        RuntimeStateStore,

    actions:
        ActionQueue,
}

impl ViewRuntime {
    pub fn new(
        component_instance:
        ComponentInstanceId,
    ) -> Self {
        Self {
            component_instance,
            tree: None,
            states:
            RuntimeStateStore::default(),
            actions:
            ActionQueue::default(),
        }
    }

    pub fn commit(
        &mut self,
        tree: ViewNode,
    ) {
        let mut active_nodes =
            HashSet::new();

        collect_node_ids(
            &tree,
            &mut active_nodes,
        );

        self.states.retain_nodes(
            &active_nodes,
        );

        self.tree =
            Some(tree);
    }

    pub fn build_view(
        &mut self,
    ) -> Option<Box<dyn View>> {
        let tree =
            self.tree.as_ref()?;

        let mut adapter =
            ViewAdapter::new(
                &mut self.states,
            );

        Some(
            adapter.build(tree),
        )
    }

    pub fn collect_actions(
        &mut self,
    ) {
        let Some(tree) =
            self.tree.as_ref()
        else {
            return;
        };

        collect_button_actions(
            tree,
            self.component_instance,
            &mut self.states,
            &mut self.actions,
        );
    }

    pub fn poll_action(
        &mut self,
    ) -> Option<RuntimeAction> {
        self.actions.poll()
    }
    
    pub fn tree(
        &mut self,
    ) -> Option<&ViewNode> {
        self.tree.as_ref()
    }
}

fn collect_node_ids(
    node: &ViewNode,
    output:
    &mut HashSet<super::NodeId>,
) {
    output.insert(node.id);

    for child in &node.children {
        collect_node_ids(
            child,
            output,
        );
    }
}

fn collect_button_actions(
    node: &ViewNode,
    component_instance:
    ComponentInstanceId,
    states:
    &mut RuntimeStateStore,
    actions:
    &mut ActionQueue,
) {
    if let ViewNodeKind::Button(
        properties,
    ) = &node.kind
    {
        let state =
            states.button(node.id);

        if state.take_clicked() {
            if let Some(action_id) =
                properties.action
            {
                actions.push(
                    RuntimeAction {
                        component_instance,
                        node_id: node.id,
                        action_id,
                        event:
                        RuntimeEvent::
                        ButtonClicked,
                    },
                );
            }
        }
    }

    for child in &node.children {
        collect_button_actions(
            child,
            component_instance,
            states,
            actions,
        );
    }
}