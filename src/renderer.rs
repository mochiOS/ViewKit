//! 描画処理の共通インターフェースを定義

use crate::draw_command::DisplayList;
use crate::geometry::{Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub logical_size: Size,
    pub physical_width: u32,
    pub physical_height: u32,
    pub scale_factor: f64,
}

impl Viewport {
    pub const fn new(
        logical_size: Size,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
    ) -> Self {
        Self {
            logical_size,
            physical_width,
            physical_height,
            scale_factor,
        }
    }

    pub const fn logical_bounds(self) -> Rect {
        Rect::new(0.0, 0.0, self.logical_size.width, self.logical_size.height)
    }
}

pub trait Renderer {
    type Error: std::error::Error + 'static;

    fn resize(&mut self, viewport: Viewport) -> Result<(), Self::Error>;

    fn render(&mut self, display_list: &DisplayList, dirty_bounds: Rect)
    -> Result<(), Self::Error>;
}
