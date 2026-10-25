use crate::geometry::Point;

pub fn rounded_rect_points(width: f32, height: f32, radius: f32, arc_steps: usize) -> Vec<Point> {
    let rect = crate::geometry::Rect::new(-width.max(0.0) / 2.0, -height.max(0.0) / 2.0, width.max(0.0), height.max(0.0));
    crate::continuous_corner::polygon(rect, [radius; 4], arc_steps.max(1))
        .into_iter().map(|(x,y)| Point::new(x,y)).collect()
}

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
