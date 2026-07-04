use crate::components::{BorderStyle, ButtonColor, RectangleColor, ZStackAlignment};
use crate::layout::{LayoutLength, StackAlignment, StackDistribution, StackGap};
use crate::theme::{Color, CornerRadius};
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
    HStack(HStackNode),
    ZStack(ZStackNode),

    Text(TextNode),
    Button(ButtonNode),

    Rectangle(RectangleNode),
    Background(RectangleNode),

    Spacer,
    Divider,

    Padding(PaddingNode),
    Frame(FrameNode),
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
pub struct HStackNode {
    pub gap: StackGap,
    pub alignment: StackAlignment,
    pub distribution: StackDistribution,
}

impl Default for HStackNode {
    fn default() -> Self {
        Self {
            gap: StackGap::Medium,
            alignment: StackAlignment::Center,
            distribution: StackDistribution::Start,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ZStackNode {
    pub alignment: ZStackAlignment,
}

impl Default for ZStackNode {
    fn default() -> Self {
        Self {
            alignment: ZStackAlignment::Center,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextNode {
    pub content: String,
    pub font_family: String,

    pub font_size: f32,
    pub line_height: f32,

    pub weight: u16,
    pub alignment: TextAlignment,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct ButtonNode {
    pub title: String,
    pub color: ButtonColor,
    pub radius: f32,
    pub action: Option<ActionId>,
}

#[derive(Clone, Copy, Debug)]
pub struct RectangleNode {
    pub color: RectangleColor,
    pub radius: CornerRadius,
    pub border: BorderStyle,
}

impl Default for RectangleNode {
    fn default() -> Self {
        Self {
            color: RectangleColor::Surface,
            radius: CornerRadius::None,
            border: BorderStyle::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PaddingNode {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameNode {
    pub width: LayoutLength,
    pub height: LayoutLength,
}
