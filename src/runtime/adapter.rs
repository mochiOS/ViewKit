use crate::components::{
    Background, Button, Divider, HStack, Padding, Rectangle, Spacer, Text, VStack, ZStack,
};
use crate::layout::{IntoStackChild, LayoutLength, StackAlignment, StackChild, StackGap};
use crate::theme::CornerRadius;
use crate::view::View;

use super::{FrameNode, RectangleNode, RuntimeStateStore, ViewNode, ViewNodeKind};

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
                    stack = stack.child(self.build_stack_child(child));
                }

                Box::new(stack)
            }

            ViewNodeKind::HStack(properties) => {
                let mut stack = HStack::new()
                    .gap(properties.gap)
                    .alignment(properties.alignment)
                    .distribution(properties.distribution);

                for child in &node.children {
                    stack = stack.child(self.build_stack_child(child));
                }

                Box::new(stack)
            }

            ViewNodeKind::ZStack(properties) => {
                let mut stack = ZStack::new().alignment(properties.alignment);

                for child in &node.children {
                    stack = stack.child(self.build_stack_child(child));
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

            ViewNodeKind::Rectangle(properties) => Box::new(build_rectangle(properties)),

            ViewNodeKind::Background(properties) => {
                let content = node
                    .children
                    .first()
                    .map(|child| self.build_embedded_child(child))
                    .unwrap_or_else(|| Box::new(VStack::new()));

                Box::new(
                    Background::new()
                        .background(build_rectangle(properties))
                        .content(content),
                )
            }

            ViewNodeKind::Padding(properties) => {
                let content = node
                    .children
                    .first()
                    .map(|child| self.build_embedded_child(child))
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

            ViewNodeKind::Frame(properties) => self.build_frame_view(node, properties),

            ViewNodeKind::Spacer | ViewNodeKind::Divider => Box::new(VStack::new()),
        }
    }

    fn build_stack_child(&mut self, node: &ViewNode) -> StackChild {
        match &node.kind {
            ViewNodeKind::Spacer => Spacer::new().into_stack_child(),

            ViewNodeKind::Divider => Divider::new().into_stack_child(),

            ViewNodeKind::Frame(properties) => self.build_frame_child(node, properties),

            _ => StackChild::new(self.build(node)),
        }
    }

    fn build_frame_child(&mut self, node: &ViewNode, properties: &FrameNode) -> StackChild {
        let child = node
            .children
            .first()
            .map(|child| self.build_stack_child(child))
            .unwrap_or_else(|| StackChild::new(VStack::new()));

        apply_frame(child, properties)
    }

    fn build_frame_view(&mut self, node: &ViewNode, properties: &FrameNode) -> Box<dyn View> {
        Box::new(
            VStack::new()
                .gap(StackGap::None)
                .alignment(StackAlignment::Start)
                .child(self.build_frame_child(node, properties)),
        )
    }

    fn build_embedded_child(&mut self, node: &ViewNode) -> Box<dyn View> {
        match &node.kind {
            ViewNodeKind::Frame(_) | ViewNodeKind::Spacer | ViewNodeKind::Divider => Box::new(
                VStack::new()
                    .gap(StackGap::None)
                    .alignment(StackAlignment::Start)
                    .child(self.build_stack_child(node)),
            ),

            _ => self.build(node),
        }
    }

    fn build_root(&mut self, node: &ViewNode) -> Box<dyn View> {
        let mut root = VStack::new();

        for child in &node.children {
            root = root.child(self.build_stack_child(child));
        }

        Box::new(root)
    }
}

fn build_rectangle(properties: &RectangleNode) -> Rectangle {
    Rectangle::new()
        .color(properties.color)
        .radius(properties.radius)
        .border(properties.border)
}

fn apply_frame(mut child: StackChild, properties: &FrameNode) -> StackChild {
    if let LayoutLength::Fixed(width) = properties.width {
        child = child.width(width);
    }

    if let LayoutLength::Fixed(height) = properties.height {
        child = child.height(height);
    }

    child
}
