//! 描画処理の共通インターフェースを定義

use crate::draw_command::DisplayList;
use crate::geometry::{Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
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

/// CPU pixels are valid only after the entire backing store has been drawn.
/// A renderer switch or allocation must never inherit a GPU damage rectangle.
pub(crate) fn raster_damage(viewport: Viewport, dirty: Rect, contents_valid: bool) -> Rect {
    let full = viewport.logical_bounds();
    if contents_valid { dirty.intersection(full).unwrap_or(full) } else { full }
}

#[cfg(test)]
mod damage_tests {
    use super::*;
    #[test]
    fn first_cpu_frame_replaces_partial_gpu_damage() {
        let viewport = Viewport::new(Size::new(100., 80.), 200, 160, 2.);
        let dirty = Rect::new(20., 20., 4., 4.);
        assert_eq!(raster_damage(viewport, dirty, false), viewport.logical_bounds());
        assert_eq!(raster_damage(viewport, dirty, true), dirty);
    }
}
