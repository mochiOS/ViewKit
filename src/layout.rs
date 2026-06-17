use ui_layout::{
    AlignItems,
    Display,
    FlexDirection,
    ItemStyle,
    JustifyContent,
    LayoutEngine,
    LayoutNode,
    Length,
    SizeStyle,
    Style,
};

use crate::geometry::{
    Point,
    Rect,
};
use crate::theme::SpacingTokens;
use crate::view::{
    PaintContext,
    View,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutLength {
    #[default]
    Auto,
    Fixed(f32),
}

impl LayoutLength {
    fn to_ui_length(self) -> Length {
        match self {
            Self::Auto => Length::Auto,
            Self::Fixed(value) => {
                Length::Px(value.max(0.0))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackAlignment {
    Start,

    #[default]
    Center,

    End,

    Stretch,
}

impl StackAlignment {
    fn to_ui_alignment(self) -> AlignItems {
        match self {
            Self::Start => AlignItems::Start,
            Self::Center => AlignItems::Center,
            Self::End => AlignItems::End,
            Self::Stretch => AlignItems::Stretch,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackDistribution {
    #[default]
    Start,

    Center,

    End,

    SpaceBetween,

    SpaceAround,

    SpaceEvenly,
}

impl StackDistribution {
    fn to_ui_justification(self) -> JustifyContent {
        match self {
            Self::Start => JustifyContent::Start,
            Self::Center => JustifyContent::Center,
            Self::End => JustifyContent::End,
            Self::SpaceBetween => {
                JustifyContent::SpaceBetween
            }
            Self::SpaceAround => {
                JustifyContent::SpaceAround
            }
            Self::SpaceEvenly => {
                JustifyContent::SpaceEvenly
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum StackGap {
    None,
    ExtraSmall,
    Small,

    #[default]
    Medium,

    Large,
    ExtraLarge,
    DoubleExtraLarge,
    Custom(f32),
}

impl StackGap {
    pub fn resolve(
        self,
        tokens: &SpacingTokens,
    ) -> f32 {
        match self {
            Self::None => 0.0,
            Self::ExtraSmall => tokens.extra_small,
            Self::Small => tokens.small,
            Self::Medium => tokens.medium,
            Self::Large => tokens.large,
            Self::ExtraLarge => tokens.extra_large,
            Self::DoubleExtraLarge => {
                tokens.double_extra_large
            }
            Self::Custom(value) => value.max(0.0),
        }
    }
}

pub struct StackChild {
    view: Box<dyn View>,
    width: LayoutLength,
    height: LayoutLength,
    flex_grow: f32,
    flex_shrink: f32,
}

impl StackChild {
    pub fn new<V>(view: V) -> Self
    where
        V: View + 'static,
    {
        Self {
            view: Box::new(view),
            width: LayoutLength::Auto,
            height: LayoutLength::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        }
    }

    pub fn width(
        mut self,
        width: f32,
    ) -> Self {
        self.width = LayoutLength::Fixed(width);
        self
    }

    pub fn height(
        mut self,
        height: f32,
    ) -> Self {
        self.height = LayoutLength::Fixed(height);
        self
    }

    pub fn frame(
        mut self,
        width: f32,
        height: f32,
    ) -> Self {
        self.width = LayoutLength::Fixed(width);
        self.height = LayoutLength::Fixed(height);
        self
    }

    pub fn flex_grow(
        mut self,
        value: f32,
    ) -> Self {
        self.flex_grow = value.max(0.0);
        self
    }

    pub fn flex_shrink(
        mut self,
        value: f32,
    ) -> Self {
        self.flex_shrink = value.max(0.0);
        self
    }

    fn layout_node(&self) -> LayoutNode {
        LayoutNode::new(
            Style {
                display: Display::Block,

                size: SizeStyle {
                    width: self.width.to_ui_length(),
                    height: self.height.to_ui_length(),
                    ..Default::default()
                },

                item_style: ItemStyle {
                    flex_grow: self.flex_grow,
                    flex_shrink: self.flex_shrink,
                    ..Default::default()
                },

                ..Default::default()
            },
        )
    }
}

pub trait ViewExt:
View + Sized + 'static
{
    fn layout(self) -> StackChild {
        StackChild::new(self)
    }

    fn frame(
        self,
        width: f32,
        height: f32,
    ) -> StackChild {
        StackChild::new(self)
            .frame(width, height)
    }

    fn width(
        self,
        width: f32,
    ) -> StackChild {
        StackChild::new(self)
            .width(width)
    }

    fn height(
        self,
        height: f32,
    ) -> StackChild {
        StackChild::new(self)
            .height(height)
    }
}

impl<T> ViewExt for T
where
    T: View + Sized + 'static,
{
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StackDirection {
    Vertical,
    Horizontal,
}

pub(crate) fn paint_stack(
    direction: StackDirection,
    children: &[StackChild],
    bounds: Rect,
    gap: StackGap,
    alignment: StackAlignment,
    distribution: StackDistribution,
    context: &mut PaintContext<'_>,
) {
    if bounds.size.width <= 0.0
        || bounds.size.height <= 0.0
    {
        return;
    }

    let gap = gap.resolve(
        &context.theme.spacing,
    );

    let child_nodes = children
        .iter()
        .map(StackChild::layout_node)
        .collect::<Vec<_>>();

    let flex_direction = match direction {
        StackDirection::Vertical => {
            FlexDirection::Column
        }
        StackDirection::Horizontal => {
            FlexDirection::Row
        }
    };

    let mut style = Style {
        display: Display::Flex {
            flex_direction,
        },

        size: SizeStyle {
            width: Length::Px(
                bounds.size.width,
            ),
            height: Length::Px(
                bounds.size.height,
            ),
            ..Default::default()
        },

        align_items: alignment
            .to_ui_alignment(),

        justify_content: distribution
            .to_ui_justification(),

        ..Default::default()
    };

    match direction {
        StackDirection::Vertical => {
            style.row_gap = Length::Px(gap);
        }

        StackDirection::Horizontal => {
            style.column_gap = Length::Px(gap);
        }
    }

    let mut root = LayoutNode::with_children(
        style,
        child_nodes,
    );

    LayoutEngine::layout(
        &mut root,
        bounds.size.width,
        bounds.size.height,
    );

    for (child, layout_node) in children
        .iter()
        .zip(root.children.iter())
    {
        let Some(child_bounds) = border_box(
            layout_node,
            bounds.origin,
        ) else {
            continue;
        };

        child.view.paint(
            child_bounds,
            context,
        );
    }
}

pub fn border_box(
    node: &LayoutNode,
    parent_origin: Point,
) -> Option<Rect> {
    let box_model = node
        .layout_boxes
        .iter()
        .next()?;

    let rect = box_model.border_box;

    Some(
        Rect::new(
            parent_origin.x + rect.x,
            parent_origin.y + rect.y,
            rect.width,
            rect.height,
        ),
    )
}