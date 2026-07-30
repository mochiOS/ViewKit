use crate::geometry::Rect;
use crate::theme::Color;

#[derive(Clone, Copy)]
pub(crate) struct ClipVertex {
    pub(crate) position: (f32, f32),
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
}

#[derive(Clone)]
pub(crate) enum ClipShape {
    Rect(Rect),
    Rounded {
        rect: Rect,
        radius: f32,
        polygon: Vec<(f32, f32)>,
    },
}

#[derive(Clone)]
pub(crate) struct ClipRegion {
    pub(crate) bounds: Rect,
    pub(crate) shapes: Vec<ClipShape>,
}

impl ClipShape {
    pub(crate) fn contains(&self, point: (f32, f32)) -> bool {
        match self {
            Self::Rect(rect) => rect_contains(*rect, point),
            Self::Rounded { rect, radius, .. } => rounded_rect_contains(*rect, *radius, point),
        }
    }

    pub(crate) fn polygon(&self) -> Vec<(f32, f32)> {
        match self {
            Self::Rect(rect) => vec![
                (rect.origin.x, rect.origin.y),
                (rect.origin.x + rect.size.width, rect.origin.y),
                (
                    rect.origin.x + rect.size.width,
                    rect.origin.y + rect.size.height,
                ),
                (rect.origin.x, rect.origin.y + rect.size.height),
            ],
            Self::Rounded { polygon, .. } => polygon.clone(),
        }
    }
}

pub(crate) fn premultiplied_color(color: Color) -> [f32; 4] {
    let alpha = f32::from(color.alpha) / 255.0;
    [
        f32::from(color.red) / 255.0 * alpha,
        f32::from(color.green) / 255.0 * alpha,
        f32::from(color.blue) / 255.0 * alpha,
        alpha,
    ]
}

pub(crate) fn clip_polygon(mut subject: Vec<ClipVertex>, clip: Vec<(f32, f32)>) -> Vec<ClipVertex> {
    if subject.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }
    let clockwise = polygon_area(&clip) >= 0.0;
    for index in 0..clip.len() {
        let start = clip[index];
        let end = clip[(index + 1) % clip.len()];
        let input = core::mem::take(&mut subject);
        let Some(mut previous) = input.last().copied() else {
            return Vec::new();
        };
        let mut previous_inside = edge_contains(start, end, previous.position, clockwise);
        for current in input {
            let current_inside = edge_contains(start, end, current.position, clockwise);
            if current_inside != previous_inside {
                subject.push(intersect_edge(previous, current, start, end));
            }
            if current_inside {
                subject.push(current);
            }
            previous = current;
            previous_inside = current_inside;
        }
        if subject.is_empty() {
            return subject;
        }
    }
    subject
}

fn rect_contains(rect: Rect, point: (f32, f32)) -> bool {
    point.0 >= rect.origin.x
        && point.1 >= rect.origin.y
        && point.0 <= rect.origin.x + rect.size.width
        && point.1 <= rect.origin.y + rect.size.height
}

fn rounded_rect_contains(rect: Rect, radius: f32, point: (f32, f32)) -> bool {
    if !rect_contains(rect, point) {
        return false;
    }
    let radius = radius
        .max(0.0)
        .min(rect.size.width.min(rect.size.height) * 0.5);
    if radius == 0.0
        || point.0 >= rect.origin.x + radius && point.0 <= rect.origin.x + rect.size.width - radius
        || point.1 >= rect.origin.y + radius && point.1 <= rect.origin.y + rect.size.height - radius
    {
        return true;
    }
    let center_x = if point.0 < rect.origin.x + radius {
        rect.origin.x + radius
    } else {
        rect.origin.x + rect.size.width - radius
    };
    let center_y = if point.1 < rect.origin.y + radius {
        rect.origin.y + radius
    } else {
        rect.origin.y + rect.size.height - radius
    };
    let dx = point.0 - center_x;
    let dy = point.1 - center_y;
    dx * dx + dy * dy <= radius * radius + 0.001
}

fn polygon_area(polygon: &[(f32, f32)]) -> f32 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(left, right)| left.0 * right.1 - right.0 * left.1)
        .sum()
}

fn edge_contains(start: (f32, f32), end: (f32, f32), point: (f32, f32), clockwise: bool) -> bool {
    let cross = (end.0 - start.0) * (point.1 - start.1) - (end.1 - start.1) * (point.0 - start.0);
    if clockwise {
        cross >= -0.001
    } else {
        cross <= 0.001
    }
}

fn intersect_edge(
    start: ClipVertex,
    end: ClipVertex,
    edge_start: (f32, f32),
    edge_end: (f32, f32),
) -> ClipVertex {
    let edge_x = edge_end.0 - edge_start.0;
    let edge_y = edge_end.1 - edge_start.1;
    let line_x = end.position.0 - start.position.0;
    let line_y = end.position.1 - start.position.1;
    let denominator = edge_x * line_y - edge_y * line_x;
    let t = if denominator.abs() <= f32::EPSILON {
        0.0
    } else {
        (edge_x * (edge_start.1 - start.position.1) - edge_y * (edge_start.0 - start.position.0))
            / denominator
    }
    .clamp(0.0, 1.0);
    ClipVertex {
        position: (start.position.0 + line_x * t, start.position.1 + line_y * t),
        uv: [
            start.uv[0] + (end.uv[0] - start.uv[0]) * t,
            start.uv[1] + (end.uv[1] - start.uv[1]) * t,
        ],
        color: core::array::from_fn(|index| {
            start.color[index] + (end.color[index] - start.color[index]) * t
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(x: f32, y: f32) -> ClipVertex {
        ClipVertex {
            position: (x, y),
            uv: [x / 10.0, y / 10.0],
            color: [x / 10.0, y / 10.0, 0.5, 1.0],
        }
    }

    #[test]
    fn rounded_clip_excludes_square_corners() {
        let rect = Rect::new(0.0, 0.0, 100.0, 60.0);
        let rectangular = ClipShape::Rect(rect);
        assert_eq!(rectangular.polygon().len(), 4);
        let region = ClipRegion {
            bounds: rect,
            shapes: vec![rectangular],
        };
        assert_eq!(region.bounds, rect);
        assert_eq!(region.shapes.len(), 1);

        let shape = ClipShape::Rounded {
            rect,
            radius: 12.0,
            polygon: vec![(12.0, 0.0), (88.0, 0.0), (100.0, 12.0)],
        };
        assert_eq!(shape.polygon().len(), 3);
        assert!(!shape.contains((0.0, 0.0)));
        assert!(shape.contains((12.0, 0.0)));
        assert!(shape.contains((50.0, 30.0)));
        assert!(!shape.contains((100.1, 30.0)));
    }

    #[test]
    fn polygon_clip_interpolates_uv_and_color() {
        let triangle = vec![vertex(-5.0, 5.0), vertex(5.0, -5.0), vertex(10.0, 10.0)];
        let clipped = clip_polygon(
            triangle,
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        );
        assert!(clipped.len() >= 3);
        for vertex in clipped {
            assert!((0.0..=10.0).contains(&vertex.position.0));
            assert!((0.0..=10.0).contains(&vertex.position.1));
            assert!((0.0..=1.0).contains(&vertex.uv[0]));
            assert!((0.0..=1.0).contains(&vertex.uv[1]));
            assert!((0.0..=1.0).contains(&vertex.color[0]));
            assert!((0.0..=1.0).contains(&vertex.color[1]));
        }
    }

    #[test]
    fn premultiplies_clip_vertex_color() {
        let color = premultiplied_color(Color::rgba(200, 100, 50, 128));
        let alpha = 128.0 / 255.0;
        assert!((color[0] - 200.0 / 255.0 * alpha).abs() < 0.0001);
        assert!((color[1] - 100.0 / 255.0 * alpha).abs() < 0.0001);
        assert!((color[2] - 50.0 / 255.0 * alpha).abs() < 0.0001);
        assert!((color[3] - alpha).abs() < 0.0001);
    }
}
