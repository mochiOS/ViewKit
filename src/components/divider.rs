//! Stack内へ区切り線を配置するDividerを定義

use crate::draw_command::DrawCommand;
use crate::geometry::Rect;
use crate::layout::{IntoStackChild, StackChild};
use crate::theme::{Color, DividerThickness};
use crate::view::{PaintContext, View};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DividerColor {
    #[default]
    Border,

    Custom(Color),
}

impl DividerColor {
    fn resolve(self, context: &PaintContext<'_>) -> Color {
        match self {
            Self::Border => context.theme.colors.border,

            Self::Custom(color) => color,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Divider {
    color: DividerColor,
    thickness: DividerThickness,
}

impl Divider {
    pub const fn new() -> Self {
        Self {
            color: DividerColor::Border,
            thickness: DividerThickness::Standard,
        }
    }

    pub const fn color(mut self, color: DividerColor) -> Self {
        self.color = color;
        self
    }

    pub const fn thickness(mut self, thickness: DividerThickness) -> Self {
        self.thickness = thickness;
        self
    }
}

impl IntoStackChild for Divider {
    fn into_stack_child(self) -> StackChild {
        StackChild::divider(DividerView { color: self.color }, self.thickness)
    }
}

struct DividerView {
    color: DividerColor,
}

impl View for DividerView {
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.display_list.push(DrawCommand::FillRect {
            rect: bounds,
            color: self.color.resolve(context),
        });
    }
}
