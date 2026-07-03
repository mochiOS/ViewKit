//! レンダラーへ渡す描画命令を定義

use crate::geometry::Rect;
use crate::theme::Color;
use crate::typography::TextAlignment;

#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    Clear {
        color: Color,
    },

    FillRect {
        rect: Rect,
        color: Color,
    },

    FillRoundedRect {
        rect: Rect,
        radius: f32,
        color: Color,
    },

    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
    },

    StrokeRoundedRect {
        rect: Rect,
        radius: f32,
        color: Color,
        width: f32,
    },

    DrawText {
        command: TextCommand,
    },

    PushClip {
        rect: Rect,
    },

    PopClip,
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    commands: Vec<DrawCommand>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextCommand {
    pub text: String,
    pub bounds: Rect,

    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: u16,
    pub alignment: TextAlignment,

    pub color: Color,
}

impl DisplayList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }
}

pub fn clamp_radius(rect: Rect, radius: f32) -> f32 {
    radius
        .max(0.0)
        .min(rect.size.width.min(rect.size.height) / 2.0)
}
