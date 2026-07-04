use crate::components::ButtonColor;
use crate::layout::{StackAlignment, StackDistribution, StackGap};
use crate::theme::Color;
use crate::typography::TextAlignment;

use super::{ActionId, NodeId};

#[derive(Clone, Debug)]
pub struct ViewNode {
    pub id: NodeId,
    pub kind: ViewNodeKind,
    pub children: Vec<ViewNode>,
}

impl ViewNode {
    pub fn new(id: NodeId, kind: ViewNodeKind) -> Self {
        Self {
            id,
            kind,
            children: Vec::new(),
        }
    }

    pub fn with_children(id: NodeId, kind: ViewNodeKind, children: Vec<ViewNode>) -> Self {
        Self { id, kind, children }
    }
}

#[derive(Clone, Debug)]
pub enum ViewNodeKind {
    Root,

    VStack(VStackNode),

    Text(TextNode),

    Button(ButtonNode),

    Padding(PaddingNode),
}

#[derive(Clone, Debug)]
pub struct VStackNode {
    pub gap: StackGap,
    pub alignment: StackAlignment,
    pub distribution: StackDistribution,
}

impl Default for VStackNode {
    fn default() -> Self {
        Self {
            gap: StackGap::Medium,
            alignment: StackAlignment::Center,
            distribution: StackDistribution::Start,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextNode {
    pub content: String,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: u16,
    pub alignment: TextAlignment,
    pub color: Color,
    pub font_family: String,
}

#[derive(Clone, Debug)]
pub struct ButtonNode {
    pub title: String,
    pub color: ButtonColor,
    pub radius: f32,
    pub action: Option<ActionId>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PaddingNode {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}
