use crate::geometry::Point;

pub fn rounded_rect_points(width: f32, height: f32, radius: f32, arc_steps: usize) -> Vec<Point> {
    let half_width = width.max(0.0) / 2.0;
    let half_height = height.max(0.0) / 2.0;
    let radius = radius.max(0.0).min(half_width.min(half_height));
    let arc_steps = arc_steps.max(1);

    if half_width == 0.0 || half_height == 0.0 {
        return Vec::new();
    }

    let corners = [
        (half_width - radius, -half_height + radius, -90.0_f32),
        (half_width - radius, half_height - radius, 0.0),
        (-half_width + radius, half_height - radius, 90.0),
        (-half_width + radius, -half_height + radius, 180.0),
    ];

    let mut points = Vec::with_capacity((arc_steps + 1) * 4);

    for (center_x, center_y, start_angle) in corners {
        for step in 0..=arc_steps {
            let angle = (start_angle + 90.0 * step as f32 / arc_steps as f32).to_radians();

            points.push(Point::new(
                center_x + angle.cos() * radius,
                center_y + angle.sin() * radius,
            ));
        }
    }

    points
}

pub fn capsule_points(half_width: f32, radius: f32, arc_steps: usize) -> Vec<Point> {
    rounded_rect_points(half_width * 2.0, radius * 2.0, radius, arc_steps)
}
