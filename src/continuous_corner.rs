//! Native continuous-corner adapters. Geometry is shared with the compositor;
//! only backend objects are cached here. Existing radius values remain setbacks.
use crate::geometry::{Point, Rect};
pub use mochios_viewkit_gpu_protocol::continuous_corner::{
    ARC_LENGTH, Cubic, RoundedRect, bezier_segments, polygon_segments, sample,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;
use tiny_skia::{Path, PathBuilder};

const CAPACITY: usize = 64;
type Key = [u32; 10];
thread_local! {
    static PATHS: RefCell<VecDeque<(Key, Arc<Path>)>> = const { RefCell::new(VecDeque::new()) };
    static POLYGONS: RefCell<VecDeque<(Key, Arc<Vec<(f32,f32)>>)>> = const { RefCell::new(VecDeque::new()) };
}
pub fn shape(rect: Rect, radii: [f32; 4]) -> RoundedRect {
    RoundedRect::new(
        [
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ],
        radii,
    )
}
pub fn contains(rect: Rect, radius: f32, point: Point) -> bool {
    shape(rect, [radius; 4]).contains((point.x, point.y))
}
fn key(shape: RoundedRect, quality: f32) -> Key {
    let b = shape.bounds;
    let r = shape.radii;
    [b[0], b[1], b[2], b[3], r[0], r[1], r[2], r[3], quality, 0.0].map(f32::to_bits)
}
fn scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// Cached cubic path; four independent radii use the CSS proportional clamp.
pub fn path(rect: Rect, radii: [f32; 4], dpi_scale: f32) -> Option<Arc<Path>> {
    let shape = shape(rect, radii);
    if shape.is_empty() {
        return None;
    }
    let n = bezier_segments(shape.max_radius() * scale(dpi_scale));
    let key = key(shape, n as f32);
    PATHS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.iter().position(|(k, _)| *k == key) {
            let entry = cache.remove(index).unwrap();
            let result = entry.1.clone();
            cache.push_back(entry);
            return Some(result);
        }
        let mut builder = PathBuilder::new();
        let mut previous_corner = usize::MAX;
        shape.for_each_cubic(n, |corner, c| {
            if previous_corner == usize::MAX {
                builder.move_to(c.start.0, c.start.1);
            } else if corner != previous_corner {
                builder.line_to(c.start.0, c.start.1);
            }
            builder.cubic_to(
                c.control1.0,
                c.control1.1,
                c.control2.0,
                c.control2.1,
                c.end.0,
                c.end.1,
            );
            previous_corner = corner;
        });
        builder.close();
        let path = Arc::new(builder.finish()?);
        if cache.len() == CAPACITY {
            cache.pop_front();
        }
        cache.push_back((key, path.clone()));
        Some(path)
    })
}

/// Cached local-space polygon. Matching segment counts must be supplied for
/// nested stroke/AA rings so corresponding vertices remain paired.
pub fn polygon(rect: Rect, radii: [f32; 4], segments: usize) -> Vec<(f32, f32)> {
    polygon_offset(rect, radii, segments, 0.0)
}

pub fn polygon_offset(
    rect: Rect,
    radii: [f32; 4],
    segments: usize,
    offset: f32,
) -> Vec<(f32, f32)> {
    if !offset.is_finite() || !rect.origin.x.is_finite() || !rect.origin.y.is_finite() {
        return Vec::new();
    }
    let local = Rect::new(0.0, 0.0, rect.size.width, rect.size.height);
    let shape = shape(local, radii);
    if shape.is_empty() {
        return Vec::new();
    }
    let mut key = key(shape, segments as f32);
    key[9] = offset.to_bits();
    POLYGONS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let points = if let Some(index) = cache.iter().position(|(k, _)| *k == key) {
            let entry = cache.remove(index).unwrap();
            let result = entry.1.clone();
            cache.push_back(entry);
            result
        } else {
            let mut points = Vec::new();
            shape.for_each_offset_point(segments, offset, |p| points.push(p));
            let result = Arc::new(points);
            if cache.len() == CAPACITY {
                cache.pop_front();
            }
            cache.push_back((key, result.clone()));
            result
        };
        points
            .iter()
            .map(|p| (p.0 + rect.origin.x, p.1 + rect.origin.y))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{FillRule, Mask, Paint, Pixmap, Stroke, Transform};
    #[test]
    fn fill_clip_stroke_and_hit_test_share_the_boundary() {
        let cases = [
            (SizeCase(90.0, 60.0), [0.0; 4]),
            (SizeCase(90.0, 60.0), [0.5; 4]),
            (SizeCase(90.0, 60.0), [10.0; 4]),
            (SizeCase(90.0, 60.0), [9999.0; 4]),
            (SizeCase(90.0, 60.0), [4.0, 18.0, 30.0, 7.0]),
        ];
        for (size, radii) in cases {
            for dpi in [1.0, 2.0, 3.0] {
                let rect = Rect::new(4.0, 4.0, size.0, size.1);
                let path = path(rect, radii, dpi).unwrap();
                let width = ((size.0 + 8.0) * dpi) as u32;
                let height = ((size.1 + 8.0) * dpi) as u32;
                let mut fill = Pixmap::new(width, height).unwrap();
                let mut paint = Paint::default();
                paint.set_color_rgba8(255, 255, 255, 255);
                let transform = Transform::from_scale(dpi, dpi);
                fill.fill_path(&path, &paint, FillRule::Winding, transform, None);
                let mut mask = Mask::new(width, height).unwrap();
                mask.fill_path(&path, FillRule::Winding, true, transform);
                for (pixel, clip) in fill.pixels().iter().zip(mask.data()) {
                    assert_eq!(pixel.alpha(), *clip);
                }
                let shape = shape(rect, radii);
                for y in 0..height {
                    for x in 0..width {
                        let p = ((x as f32 + 0.5) / dpi, (y as f32 + 0.5) / dpi);
                        let alpha = fill.pixel(x, y).unwrap().alpha();
                        // Fully covered/empty pixels must agree except within an
                        // AA fringe; test points 0.6 physical pixels away too.
                        if alpha == 255 {
                            assert!(shape.contains(p), "{p:?} {radii:?}");
                        }
                        if alpha == 0 && shape.contains(p) {
                            let mut neighbours = false;
                            for (dx, dy) in [(0.6, 0.0), (-0.6, 0.0), (0.0, 0.6), (0.0, -0.6)] {
                                neighbours |= !shape.contains((p.0 + dx / dpi, p.1 + dy / dpi));
                            }
                            assert!(neighbours);
                        }
                    }
                }
                let mut stroke = Pixmap::new(width, height).unwrap();
                stroke.stroke_path(
                    &path,
                    &paint,
                    &Stroke {
                        width: 1.0,
                        ..Stroke::default()
                    },
                    transform,
                    None,
                );
                // Stroke straddles the same contour; both sides are visible.
                assert!(stroke.pixels().iter().any(|p| p.alpha() > 0));
                assert!(stroke.pixel(width / 2, height / 2).unwrap().alpha() == 0);
            }
        }
    }
    struct SizeCase(f32, f32);
    #[test]
    fn cached_paths_are_reused_and_resize_storage_is_bounded() {
        let rect = Rect::new(3.0, 5.0, 100.0, 30.0);
        let a = path(rect, [9.0; 4], 2.0).unwrap();
        let b = path(rect, [9.0; 4], 2.0).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        for i in 0..256 {
            let rect = Rect::new(0.0, 0.0, 100.0 + i as f32, 30.0);
            assert!(path(rect, [9.0; 4], 2.0).is_some());
            assert!(!polygon(rect, [9.0; 4], 16).is_empty());
        }
        PATHS.with(|c| assert_eq!(c.borrow().len(), CAPACITY));
        POLYGONS.with(|c| assert_eq!(c.borrow().len(), CAPACITY));
    }
    #[test]
    fn normal_offsets_preserve_capsule_joins_and_border_width() {
        let rect = Rect::new(0.0, 0.0, 120.0, 30.0);
        let n = 32;
        let base = polygon(rect, [15.0; 4], n);
        let outer = polygon_offset(rect, [15.0; 4], n, 0.5);
        let inner = polygon_offset(rect, [15.0; 4], n, -0.5);
        for ((p, o), i) in base.iter().zip(&outer).zip(&inner) {
            assert!(((o.0 - p.0).hypot(o.1 - p.1) - 0.5).abs() < 0.00002);
            assert!(((o.0 - i.0).hypot(o.1 - i.1) - 1.0).abs() < 0.00002);
        }
        assert_eq!(outer[n], outer[n + 1]);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    #[ignore = "manual release-mode geometry timing, not a timing assertion"]
    fn compare_cached_continuous_geometry_to_legacy_arc_generation() {
        use std::{hint::black_box, time::Instant};
        let rounds = 50_000;
        let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
        let start = Instant::now();
        for _ in 0..rounds {
            let mut points = Vec::with_capacity(48);
            for (cx, cy, angle) in [
                (91.0, 9.0, -core::f32::consts::FRAC_PI_2),
                (91.0, 21.0, 0.0),
                (9.0, 21.0, core::f32::consts::FRAC_PI_2),
                (9.0, 9.0, core::f32::consts::PI),
            ] {
                for step in 0..12 {
                    let a = angle + core::f32::consts::FRAC_PI_2 * step as f32 / 11.0;
                    points.push((cx + 9.0 * a.cos(), cy + 9.0 * a.sin()));
                }
            }
            black_box(points);
        }
        let legacy = start.elapsed();
        let n = polygon_segments(9.0);
        black_box(polygon(bounds, [9.0; 4], n));
        let start = Instant::now();
        for _ in 0..rounds {
            black_box(polygon(bounds, [9.0; 4], n));
        }
        let continuous = start.elapsed();
        std::println!(
            "{rounds} polygons: legacy {legacy:?} (48 vertices), cached continuous {continuous:?} ({} vertices); normalized data=32800 bytes; polygon cache <= 2099200 coordinate bytes per thread",
            4 * (n + 1)
        );
    }
}

/// Inside-aligned border: preserve the exact outer path and offset only its
/// inner contour. Subtracting a radius and insetting a rectangle is not a
/// parallel curve for continuous corners.
pub fn inside_border_path(
    rect: Rect,
    radius: f32,
    width: f32,
    dpi_scale: f32,
) -> Option<Arc<Path>> {
    inside_border_path_corners(rect, [radius; 4], width, dpi_scale)
}

pub fn inside_border_path_corners(
    rect: Rect,
    radii: [f32; 4],
    width: f32,
    dpi_scale: f32,
) -> Option<Arc<Path>> {
    let radius = shape(rect, radii).max_radius();
    if !width.is_finite() || width <= 0.0 {
        return None;
    }
    let outer = path(rect, radii, dpi_scale)?;
    if width >= rect.size.width.min(rect.size.height) * 0.5 {
        return Some(outer);
    }
    let n = polygon_segments(radius * scale(dpi_scale));
    let mut cache_key = key(
        shape(rect, radii),
        bezier_segments(radius * scale(dpi_scale)) as f32,
    );
    cache_key[9] = width.to_bits();
    PATHS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.iter().position(|(k, _)| *k == cache_key) {
            let entry = cache.remove(index).unwrap();
            let result = entry.1.clone();
            cache.push_back(entry);
            return Some(result);
        }
        let inner = polygon_offset(rect, radii, n, -width);
        let mut builder = PathBuilder::new();
        builder.push_path(&outer);
        if let Some(first) = inner.first() {
            builder.move_to(first.0, first.1);
            for p in &inner[1..] {
                builder.line_to(p.0, p.1);
            }
            builder.close();
        }
        let result = Arc::new(builder.finish()?);
        if cache.len() == CAPACITY {
            cache.pop_front();
        }
        cache.push_back((cache_key, result.clone()));
        Some(result)
    })
}

#[cfg(test)]
mod inside_border_tests {
    use super::*;
    #[test]
    fn inside_border_preserves_outer_silhouette_at_small_and_large_radii() {
        use tiny_skia::{FillRule, Paint, Pixmap, Transform};
        let rect = Rect::new(4.0, 4.0, 80.0, 40.0);
        let mut paint = Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        for radius in [0.1, 0.5, 1.0, 4.0, 10.0, 20.0] {
            let mut fill = Pixmap::new(176, 96).unwrap();
            fill.fill_path(
                &path(rect, [radius; 4], 2.0).unwrap(),
                &paint,
                FillRule::Winding,
                Transform::from_scale(2.0, 2.0),
                None,
            );
            for width in [0.5, 1.0, 2.0, 8.0] {
                let border = inside_border_path(rect, radius, width, 2.0).unwrap();
                let mut pixels = Pixmap::new(176, 96).unwrap();
                pixels.fill_path(
                    &border,
                    &paint,
                    FillRule::EvenOdd,
                    Transform::from_scale(2.0, 2.0),
                    None,
                );
                for (i, (a, b)) in fill.pixels().iter().zip(pixels.pixels()).enumerate() {
                    assert!(
                        b.alpha() <= a.alpha(),
                        "radius={radius} width={width} index={i}: border={}, fill={}",
                        b.alpha(),
                        a.alpha()
                    );
                }
                assert_eq!(pixels.pixel(88, 48).unwrap().alpha(), 0);
            }
        }
    }
}
