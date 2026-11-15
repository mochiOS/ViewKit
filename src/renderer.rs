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

/// Converts logical coordinates to the physical pixel grid. Text origins and
/// baselines use the same rounding rule on CPU and GPU; glyph advances retain
/// their subpixel positions. Invalid scale values resolve to 1×.
#[derive(Clone, Copy, Debug)]
pub struct PixelGrid {
    scale: f32,
}
impl PixelGrid {
    /// Creates a grid from a viewport's scale factor.
    pub fn new(scale: f64) -> Self {
        Self {
            scale: if scale.is_finite() && scale > 0.0 {
                (scale as f32).clamp(0.01, 64.0)
            } else {
                1.0
            },
        }
    }
    /// Rounds a logical coordinate to an integer physical pixel.
    pub fn physical(self, logical: f32) -> f32 {
        (logical * self.scale).round()
    }
    /// Returns the logical width of a one-physical-pixel hairline.
    pub fn hairline(self) -> f32 {
        1.0 / self.scale
    }
    /// Produces at most nine pixel-aligned rectangles with box coverage. This
    /// preserves fractional thin lines on targets without multisampling.
    pub(crate) fn coverage(self, rect: Rect) -> Vec<(Rect, f32)> {
        fn spans(start: f32, end: f32) -> Vec<(f32, f32, f32)> {
            if !start.is_finite() || !end.is_finite() || end <= start {
                return Vec::new();
            }
            let left = start.floor();
            let right = end.ceil();
            if right - left <= 1.0 {
                return vec![(left, right, end - start)];
            }
            let inner_start = start.ceil();
            let inner_end = end.floor();
            let mut spans = Vec::new();
            if inner_start > left {
                spans.push((left, inner_start, inner_start - start));
            }
            if inner_end > inner_start {
                spans.push((inner_start, inner_end, 1.0));
            }
            if right > inner_end {
                spans.push((inner_end, right, end - inner_end));
            }
            spans
        }
        let xs = spans(
            rect.origin.x * self.scale,
            (rect.origin.x + rect.size.width) * self.scale,
        );
        let ys = spans(
            rect.origin.y * self.scale,
            (rect.origin.y + rect.size.height) * self.scale,
        );
        let mut output = Vec::with_capacity(xs.len() * ys.len());
        for (x0, x1, xc) in xs {
            for &(y0, y1, yc) in &ys {
                output.push((
                    Rect::new(
                        x0 / self.scale,
                        y0 / self.scale,
                        (x1 - x0) / self.scale,
                        (y1 - y0) / self.scale,
                    ),
                    xc * yc,
                ));
            }
        }
        output
    }
}
#[cfg(test)]
mod pixel_grid_tests {
    use super::*;
    #[test]
    fn fractional_lines_preserve_physical_area_at_all_scales() {
        for scale in [0.75, 1.0, 1.25, 1.5, 2.0] {
            let grid = PixelGrid::new(scale);
            for width in [0.2, 1.0, 1.5, 20.0] {
                let rect = Rect::new(0.35, 1.125, width, 4.25);
                let spans = grid.coverage(rect);
                assert!(spans.len() <= 9);
                let area: f32 = spans
                    .iter()
                    .map(|(r, c)| r.size.width * r.size.height * c)
                    .sum();
                assert!((area - rect.size.width * rect.size.height).abs() < 0.0001);
                assert!(spans.iter().all(|(_, c)| *c > 0.0 && *c <= 1.0));
            }
            assert!((grid.hairline() * scale as f32 - 1.0).abs() < 0.0001);
        }
        assert_eq!(PixelGrid::new(f64::NAN).physical(1.2), 1.0);
    }
}
