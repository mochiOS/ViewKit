//! 矩形コンポーネントを定義

use crate::draw_command::DrawCommand;
use crate::geometry::Rect;
use crate::theme::{
    Color,
    CornerRadius,
};
use crate::view::{
    PaintContext,
    View,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RectangleColor {
    Background,
    Surface,
    ElevatedSurface,
    Accent,
    Destructive,
    Custom(Color),
}

impl RectangleColor {
    fn resolve(
        self,
        context: &PaintContext<'_>,
    ) -> Color {
        match self {
            Self::Background => {
                context.theme.colors.background
            }

            Self::Surface => {
                context.theme.colors.surface
            }

            Self::ElevatedSurface => {
                context.theme.colors.elevated_surface
            }

            Self::Accent => {
                context.theme.colors.accent
            }

            Self::Destructive => {
                context.theme.colors.destructive
            }

            Self::Custom(color) => color,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    color: RectangleColor,
    radius: CornerRadius,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            color: RectangleColor::Surface,
            radius: CornerRadius::None,
        }
    }
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(
        mut self,
        color: RectangleColor,
    ) -> Self {
        self.color = color;
        self
    }

    pub fn radius(
        mut self,
        radius: CornerRadius,
    ) -> Self {
        self.radius = radius;
        self
    }
}

impl View for Rectangle {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        let color = self.color.resolve(context);

        let radius = self.radius.resolve(
            &context.theme.radius,
            bounds.size.width,
            bounds.size.height,
        );

        if radius > 0.0 {
            context.display_list.push(
                DrawCommand::FillRoundedRect {
                    rect: bounds,
                    radius,
                    color,
                },
            );
        } else {
            context.display_list.push(
                DrawCommand::FillRect {
                    rect: bounds,
                    color,
                },
            );
        }
    }
}