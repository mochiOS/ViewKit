//! Viewの描画スタイルを定義

use crate::theme::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    pub color: Color,
    pub width: f32,
}

impl Border {
    pub const fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}