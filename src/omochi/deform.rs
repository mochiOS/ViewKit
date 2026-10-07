use crate::geometry::Point;

use super::material::OmochiMaterial;

pub fn clamp_vector(x: f32, y: f32, maximum_length: f32) -> Point {
    let length = x.hypot(y);

    if length <= maximum_length {
        return Point::new(x, y);
    }

    if length < 0.0001 {
        return Point::new(0.0, 0.0);
    }

    let scale = maximum_length / length;

    Point::new(x * scale, y * scale)
}

pub fn gaussian(distance_squared: f32, radius: f32) -> f32 {
    (-distance_squared / (2.0 * radius * radius)).exp()
}

pub fn smoothstep(value: f32) -> f32 {
    let x = value.clamp(0.0, 1.0);

    x * x * (3.0 - 2.0 * x)
}

pub fn deform_points(
    base_points: &[Point],
    pull: Point,
    press_depth: f32,
    anchor: Point,
    material: OmochiMaterial,
) -> Vec<Point> {
    if base_points.is_empty() {
        return Vec::new();
    }

    let pull_length = pull.x.hypot(pull.y);
    let has_pull = pull_length > 0.001;

    let direction_x = if has_pull { pull.x / pull_length } else { 0.0 };
    let direction_y = if has_pull { pull.y / pull_length } else { 0.0 };

    let perpendicular_x = -direction_y;
    let perpendicular_y = direction_x;

    let mut minimum_x = f32::INFINITY;
    let mut maximum_x = f32::NEG_INFINITY;
    let mut minimum_y = f32::INFINITY;
    let mut maximum_y = f32::NEG_INFINITY;

    for point in base_points {
        minimum_x = minimum_x.min(point.x);
        maximum_x = maximum_x.max(point.x);
        minimum_y = minimum_y.min(point.y);
        maximum_y = maximum_y.max(point.y);
    }

    let width = (maximum_x - minimum_x).max(1.0);
    let height = (maximum_y - minimum_y).max(1.0);

    let axis_extent = if has_pull {
        (direction_x.abs() * width / 2.0 + direction_y.abs() * height / 2.0).max(1.0)
    } else {
        1.0
    };

    let cross_extent = if has_pull {
        (perpendicular_x.abs() * width / 2.0 + perpendicular_y.abs() * height / 2.0).max(1.0)
    } else {
        1.0
    };

    let transition_scale = (axis_extent * 0.42).max(18.0);

    let mut surface_along = f32::NEG_INFINITY;

    if has_pull {
        for point in base_points {
            let relative_x = point.x - anchor.x;
            let relative_y = point.y - anchor.y;
            let along = relative_x * direction_x + relative_y * direction_y;

            surface_along = surface_along.max(along);
        }
    }

    let mut drag_offsets = Vec::with_capacity(base_points.len());

    for point in base_points {
        if !has_pull {
            drag_offsets.push(Point::new(0.0, 0.0));
            continue;
        }

        let relative_x = point.x - anchor.x;
        let relative_y = point.y - anchor.y;
        let along = relative_x * direction_x + relative_y * direction_y;
        let distance_squared = relative_x * relative_x + relative_y * relative_y;
        let forward = 0.5 + 0.5 * (along / transition_scale).tanh();
        let local = gaussian(distance_squared, material.drag_radius);
        let follow = material.drag_follow * forward * (0.72 + 0.28 * local);

        drag_offsets.push(Point::new(pull.x * follow, pull.y * follow));
    }

    let mut mean = Point::new(0.0, 0.0);

    for offset in &drag_offsets {
        mean.x += offset.x;
        mean.y += offset.y;
    }

    mean.x /= drag_offsets.len() as f32;
    mean.y /= drag_offsets.len() as f32;

    base_points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let mut x = point.x;
            let mut y = point.y;

            let to_anchor_x = anchor.x - point.x;
            let to_anchor_y = anchor.y - point.y;
            let anchor_distance_squared = to_anchor_x * to_anchor_x + to_anchor_y * to_anchor_y;
            let anchor_distance = anchor_distance_squared.sqrt();
            let press_influence = gaussian(anchor_distance_squared, material.press_radius);

            let release_range =
                (material.press_release_end - material.press_release_start).max(0.001);
            let release_raw = (pull_length - material.press_release_start) / release_range;
            let drag_release = smoothstep(release_raw);
            let effective_press_depth = press_depth * (1.0 - drag_release);

            if anchor_distance > 0.001 && effective_press_depth > 0.001 {
                x += to_anchor_x / anchor_distance * effective_press_depth * press_influence;
                y += to_anchor_y / anchor_distance * effective_press_depth * press_influence;
            }

            if has_pull {
                let offset = drag_offsets[index];

                x += offset.x - mean.x * material.center_lock;
                y += offset.y - mean.y * material.center_lock;

                let relative_x = point.x - anchor.x;
                let relative_y = point.y - anchor.y;
                let along = relative_x * direction_x + relative_y * direction_y;
                let across = relative_x * perpendicular_x + relative_y * perpendicular_y;

                let behind_tip = surface_along - along;
                let tip_long = (-(behind_tip * behind_tip)
                    / (2.0 * material.tip_long_radius * material.tip_long_radius))
                    .exp();
                let tip_cross = (-(across * across)
                    / (2.0 * material.tip_cross_radius * material.tip_cross_radius))
                    .exp();
                let tip_influence = tip_long * tip_cross;

                x += pull.x * material.tip_stretch * tip_influence;
                y += pull.y * material.tip_stretch * tip_influence;

                let neck_width = (axis_extent * material.neck_width_ratio).max(14.0);
                let neck_center = surface_along - axis_extent * material.neck_backshift_ratio;
                let neck_along = along - neck_center;
                let neck = (-(neck_along * neck_along) / (2.0 * neck_width * neck_width)).exp();

                let across_amount = (across.abs() / cross_extent).clamp(0.0, 1.0);
                let contraction =
                    pull_length * material.neck_ratio * neck * across_amount.powf(0.76);

                let side = if across > 0.0 {
                    1.0
                } else if across < 0.0 {
                    -1.0
                } else {
                    0.0
                };

                x -= perpendicular_x * side * contraction;
                y -= perpendicular_y * side * contraction;
            }

            Point::new(x, y)
        })
        .collect()
}
