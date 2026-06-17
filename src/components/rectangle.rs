//! 矩形コンポーネント

use crate::draw_command::DrawCommand;
use crate::geometry::Rect;
use crate::theme::Color;
use crate::view::{
    PaintContext,
    View,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    pub color: Color,
    pub radius: f32,
}

impl Rectangle {
    pub const fn new(color: Color) -> Self {
        Self {
            color,
            radius: 0.0,
        }
    }

    pub const fn rounded(
        color: Color,
        radius: f32,
    ) -> Self {
        Self {
            color,
            radius,
        }
    }
}

impl View for Rectangle {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        if self.radius > 0.0 {
            context.display_list.push(
                DrawCommand::FillRoundedRect {
                    rect: bounds,
                    radius: self.radius,
                    color: self.color,
                },
            );
        } else {
            context.display_list.push(
                DrawCommand::FillRect {
                    rect: bounds,
                    color: self.color,
                },
            );
        }
    }
}