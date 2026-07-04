use crate::components::{Button, HStack, Padding, Text, VStack, ZStack};
use crate::theme::CornerRadius;
use crate::view::View;

use super::{RuntimeStateStore, ViewNode, ViewNodeKind};

pub struct ViewAdapter<'a> {
    states: &'a mut RuntimeStateStore,
}

impl<'a> ViewAdapter<'a> {
    pub fn new(states: &'a mut RuntimeStateStore) -> Self {
        Self { states }
    }

    pub fn build(&mut self, node: &ViewNode) -> Box<dyn View> {
        match &node.kind {
            ViewNodeKind::Root => self.build_root(node),

            ViewNodeKind::VStack(properties) => {
                let mut stack = VStack::new()
                    .gap(properties.gap)
                    .alignment(properties.alignment)
                    .distribution(properties.distribution);

                for child in &node.children {
                    stack = stack.child(self.build(child));
                }

                Box::new(stack)
            }

            ViewNodeKind::HStack(properties) => {
                let mut stack = HStack::new()
                    .gap(properties.gap)
                    .alignment(properties.alignment)
                    .distribution(properties.distribution);

                for child in &node.children {
                    stack = stack.child(self.build(child));
                }

                Box::new(stack)
            }

            ViewNodeKind::ZStack(properties) => {
                let mut stack = ZStack::new().alignment(properties.alignment);

                for child in &node.children {
                    stack = stack.child(self.build(child));
                }

                Box::new(stack)
            }

            ViewNodeKind::Text(properties) => Box::new(
                Text::new(properties.content.clone())
                    .font_family(properties.font_family.clone())
                    .font_size(properties.font_size)
                    .line_height(properties.line_height)
                    .weight(properties.weight)
                    .alignment(properties.alignment)
                    .color(properties.color),
            ),

            ViewNodeKind::Button(properties) => {
                let state = self.states.button(node.id);

                Box::new(
                    Button::with_interaction(state)
                        .color(properties.color)
                        .radius(CornerRadius::Custom(properties.radius.max(0.0)))
                        .content(Text::new(properties.title.clone())),
                )
            }

            ViewNodeKind::Padding(properties) => {
                let content = node
                    .children
                    .first()
                    .map(|child| self.build(child))
                    .unwrap_or_else(|| Box::new(VStack::new()));

                Box::new(
                    Padding::only(
                        properties.top,
                        properties.right,
                        properties.bottom,
                        properties.left,
                    )
                    .content(content),
                )
            }
        }
    }

    fn build_root(&mut self, node: &ViewNode) -> Box<dyn View> {
        let mut root = VStack::new();

        for child in &node.children {
            root = root.child(self.build(child));
        }

        Box::new(root)
    }
}
