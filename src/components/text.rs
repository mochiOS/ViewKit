//! 文字列を描画するTextコンポーネント

use crate::draw_command::{
    DrawCommand,
    TextCommand,
};
use crate::geometry::Rect;
use crate::theme::Color;
use crate::view::{
    PaintContext,
    View,
};

pub struct Text {
    value: String,

    font_family: String,
    font_size: f32,
    line_height: f32,
    weight: u16,

    color: Color,
}

impl Text {
    pub fn new(
        value: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),

            font_family: String::from(
                "Noto Sans JP",
            ),

            font_size: 16.0,
            line_height: 24.0,
            weight: 400,

            color: Color::BLACK,
        }
    }

    pub fn font_family(
        mut self,
        font_family: impl Into<String>,
    ) -> Self {
        self.font_family =
            font_family.into();

        self
    }

    pub fn font_size(
        mut self,
        font_size: f32,
    ) -> Self {
        self.font_size =
            finite_positive_or(
                font_size,
                16.0,
            );

        self
    }

    pub fn line_height(
        mut self,
        line_height: f32,
    ) -> Self {
        self.line_height =
            finite_positive_or(
                line_height,
                self.font_size,
            );

        self
    }

    pub fn weight(
        mut self,
        weight: u16,
    ) -> Self {
        self.weight =
            weight.clamp(
                1,
                1000,
            );

        self
    }

    pub fn color(
        mut self,
        color: Color,
    ) -> Self {
        self.color = color;
        self
    }
}

impl View for Text {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        if bounds.size.width <= 0.0
            || bounds.size.height <= 0.0
            || self.value.is_empty()
        {
            return;
        }

        context.display_list.push(
            DrawCommand::PushClip {
                rect: bounds,
            },
        );

        context.display_list.push(
            DrawCommand::DrawText {
                command: TextCommand {
                    text: self.value.clone(),
                    bounds,

                    font_family:
                    self.font_family.clone(),

                    font_size:
                    self.font_size,

                    line_height:
                    self.line_height,

                    weight:
                    self.weight,

                    color:
                    self.color,
                },
            },
        );

        context.display_list.push(
            DrawCommand::PopClip,
        );
    }
}

fn finite_positive_or(
    value: f32,
    fallback: f32,
) -> f32 {
    if value.is_finite()
        && value > 0.0
    {
        value
    } else {
        fallback
    }
}