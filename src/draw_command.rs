//! レンダラーへ渡す描画命令を定義

use crate::geometry::{Point, Rect};
use crate::theme::Color;
use crate::typography::TextStyle;

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
        position: Point,
        text: String,
        style: TextStyle,
        color: Color,
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