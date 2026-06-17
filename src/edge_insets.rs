//! 上下左右の余白を定義

use crate::geometry::Size;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self::all(0.0);

    pub const fn new(
        top: f32,
        right: f32,
        bottom: f32,
        left: f32,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }

    pub const fn horizontal(value: f32) -> Self {
        Self::symmetric(value, 0.0)
    }

    pub const fn vertical(value: f32) -> Self {
        Self::symmetric(0.0, value)
    }

    pub const fn horizontal_sum(self) -> f32 {
        self.left + self.right
    }

    pub const fn vertical_sum(self) -> f32 {
        self.top + self.bottom
    }

    pub fn inset_size(self, size: Size) -> Size {
        Size::new(
            (size.width - self.horizontal_sum()).max(0.0),
            (size.height - self.vertical_sum()).max(0.0),
        )
    }

    pub fn outset_size(self, size: Size) -> Size {
        Size::new(
            size.width + self.horizontal_sum(),
            size.height + self.vertical_sum(),
        )
    }
}