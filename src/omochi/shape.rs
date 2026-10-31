use crate::geometry::Point;

/// Builds a centered Continuous Rounded Corner contour for material deformation.
/// `radius` is the straight-edge setback. `arc_steps` remains a minimum quality
/// hint; the shared error budget can require more segments for a large radius.
/// Invalid dimensions produce an empty contour; invalid radii become zero.
pub fn rounded_rect_points(width: f32, height: f32, radius: f32, arc_steps: usize) -> Vec<Point> {
    let rect = crate::geometry::Rect::new(
        -width.max(0.0) / 2.0,
        -height.max(0.0) / 2.0,
        width.max(0.0),
        height.max(0.0),
    );
    let shape = crate::continuous_corner::shape(rect, [radius; 4]);
    // A caller's historical arc step count is a minimum, not permission to
    // coarsen a large mathematical corner below the shared error budget.
    let segments = arc_steps.max(crate::continuous_corner::polygon_segments(
        shape.max_radius(),
    ));
    crate::continuous_corner::polygon(rect, shape.radii, segments)
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect()
}

/// Builds a maximum-radius continuous rounded rectangle, rather than a pair of
/// circular end caps. The same normalization and quality rules apply as above.
pub fn capsule_points(half_width: f32, radius: f32, arc_steps: usize) -> Vec<Point> {
    rounded_rect_points(half_width * 2.0, radius * 2.0, radius, arc_steps)
}

pub fn smooth_closed_points(points: &[Point], tension: f32, subdivisions: usize) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let tension = tension.clamp(0.0, 1.0);
    let subdivisions = subdivisions.max(1);
    let count = points.len();
    let mut output = Vec::with_capacity(count * subdivisions);

    for index in 0..count {
        let previous = points[(index + count - 1) % count];
        let current = points[index];
        let next = points[(index + 1) % count];
        let after_next = points[(index + 2) % count];

        let control1 = Point::new(
            current.x + (next.x - previous.x) * tension / 6.0,
            current.y + (next.y - previous.y) * tension / 6.0,
        );
        let control2 = Point::new(
            next.x - (after_next.x - current.x) * tension / 6.0,
            next.y - (after_next.y - current.y) * tension / 6.0,
        );

        for step in 0..subdivisions {
            let t = step as f32 / subdivisions as f32;
            let inverse = 1.0 - t;
            let inverse_squared = inverse * inverse;
            let t_squared = t * t;

            output.push(Point::new(
                inverse_squared * inverse * current.x
                    + 3.0 * inverse_squared * t * control1.x
                    + 3.0 * inverse * t_squared * control2.x
                    + t_squared * t * next.x,
                inverse_squared * inverse * current.y
                    + 3.0 * inverse_squared * t * control1.y
                    + 3.0 * inverse * t_squared * control2.y
                    + t_squared * t * next.y,
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_rounded_surfaces_cannot_request_a_coarse_legacy_outline() {
        for radius in [1.0, 16.0, 100.0, 1000.0] {
            let points = rounded_rect_points(radius * 4.0, radius * 2.0, radius, 1);
            let reference = crate::continuous_corner::shape(
                crate::geometry::Rect::new(-radius * 2.0, -radius, radius * 4.0, radius * 2.0),
                [radius; 4],
            );
            // Dense reference samples must remain within the shared 0.1px
            // budget of the actual polyline used as the deformation base.
            reference.for_each_point(1024, |(x, y)| {
                let distance = points
                    .iter()
                    .zip(points.iter().cycle().skip(1))
                    .map(|(a, b)| {
                        let dx = b.x - a.x;
                        let dy = b.y - a.y;
                        let length_squared = dx * dx + dy * dy;
                        let t = if length_squared > 0.0 {
                            ((x - a.x) * dx + (y - a.y) * dy) / length_squared
                        } else {
                            0.0
                        }
                        .clamp(0.0, 1.0);
                        (x - a.x - t * dx).hypot(y - a.y - t * dy)
                    })
                    .fold(f32::INFINITY, f32::min);
                assert!(distance <= 0.1, "radius {radius}, error {distance}");
            });
        }
    }

    #[test]
    fn capsules_and_invalid_bounds_use_the_shared_normalization() {
        assert_eq!(
            capsule_points(40.0, 12.0, 8),
            rounded_rect_points(80.0, 24.0, 12.0, 8)
        );
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(rounded_rect_points(width, 40.0, 10.0, 8).is_empty());
        }
        for radius in [-1.0, f32::NAN, f32::INFINITY] {
            let points = rounded_rect_points(40.0, 20.0, radius, 8);
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
            assert!(
                points
                    .iter()
                    .all(|p| p.x.abs() == 20.0 && p.y.abs() == 10.0)
            );
        }
    }
}
