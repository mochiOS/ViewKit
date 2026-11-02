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
        radii: [f32; 4],
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
            Self::Rounded { rect, radii, .. } => {
                crate::continuous_corner::shape(*rect, *radii).contains(point)
            }
        }
    }

    #[cfg(test)]
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

pub(crate) fn clip_polygon(mut subject: Vec<ClipVertex>, clip: &[(f32, f32)]) -> Vec<ClipVertex> {
    if subject.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }
    let clockwise = polygon_area(&clip) >= 0.0;
    let mut scratch = Vec::new();
    for index in 0..clip.len() {
        let start = clip[index];
        let end = clip[(index + 1) % clip.len()];
        // Convex clips usually leave an interior triangle unchanged. Keep the
        // existing allocation and attributes instead of copying it per edge.
        let inside_count = subject
            .iter()
            .filter(|vertex| edge_contains(start, end, vertex.position, clockwise))
            .count();
        if inside_count == subject.len() {
            continue;
        }
        if inside_count == 0 {
            subject.clear();
            return subject;
        }
        let Some(mut previous) = subject.last().copied() else {
            return subject;
        };
        scratch.clear();
        let mut previous_inside = edge_contains(start, end, previous.position, clockwise);
        for current in subject.iter().copied() {
            let current_inside = edge_contains(start, end, current.position, clockwise);
            if current_inside != previous_inside {
                scratch.push(intersect_edge(previous, current, start, end));
            }
            if current_inside {
                scratch.push(current);
            }
            previous = current;
            previous_inside = current_inside;
        }
        core::mem::swap(&mut subject, &mut scratch);
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
    // A fixed cross-product epsilon grows into a large distance error on
    // subpixel corner edges. Scale it with edge length instead.
    let epsilon = 0.00001 * ((end.0 - start.0).abs() + (end.1 - start.1).abs());
    if clockwise {
        cross >= -epsilon
    } else {
        cross <= epsilon
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
            radii: [12.0; 4],
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
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
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

#[cfg(test)]
mod optimization_tests {
    use super::*;
    fn reference_clip_polygon(
        mut subject: Vec<ClipVertex>,
        clip: &[(f32, f32)],
    ) -> Vec<ClipVertex> {
        if subject.len() < 3 || clip.len() < 3 {
            return Vec::new();
        }
        let clockwise = polygon_area(&clip) >= 0.0;
        let mut scratch = Vec::new();
        for index in 0..clip.len() {
            let start = clip[index];
            let end = clip[(index + 1) % clip.len()];
            let Some(mut previous) = subject.last().copied() else {
                return subject;
            };
            scratch.clear();
            let mut previous_inside = edge_contains(start, end, previous.position, clockwise);
            for current in subject.iter().copied() {
                let current_inside = edge_contains(start, end, current.position, clockwise);
                if current_inside != previous_inside {
                    scratch.push(intersect_edge(previous, current, start, end));
                }
                if current_inside {
                    scratch.push(current);
                }
                previous = current;
                previous_inside = current_inside;
            }
            core::mem::swap(&mut subject, &mut scratch);
            if subject.is_empty() {
                return subject;
            }
        }
        subject
    }

    fn vertices(points: &[(f32, f32)]) -> Vec<ClipVertex> {
        points
            .iter()
            .map(|&position| ClipVertex {
                position,
                uv: [position.0 * 0.01, position.1 * 0.02],
                color: [0.2, 0.4, position.0 * 0.001, 0.8],
            })
            .collect()
    }

    #[test]
    fn fast_clipping_matches_reference_including_interpolated_attributes() {
        let shape = crate::continuous_corner::shape(Rect::new(0., 0., 160., 100.), [26.5; 4]);
        let mut clip = Vec::new();
        shape.for_each_point(32, |p| clip.push(p));
        for reverse in [false, true] {
            if reverse {
                clip.reverse();
            }
            for x in (-20..180).step_by(7) {
                for y in (-20..120).step_by(7) {
                    let triangle = vertices(&[
                        (x as f32, y as f32),
                        (x as f32 + 17.25, y as f32 - 5.),
                        (x as f32 + 8., y as f32 + 22.5),
                    ]);
                    let expected = reference_clip_polygon(triangle.clone(), &clip);
                    let actual = clip_polygon(triangle, &clip);
                    let attributes = |v: Vec<ClipVertex>| {
                        v.into_iter()
                            .map(|v| (v.position, v.uv, v.color))
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(attributes(actual), attributes(expected));
                }
            }
        }
    }

    #[test]
    #[ignore = "manual clipping CPU benchmark"]
    fn benchmark_mostly_interior_edges() {
        let shape = crate::continuous_corner::shape(Rect::new(0., 0., 800., 600.), [26.5; 4]);
        let mut clip = Vec::new();
        shape.for_each_point(32, |p| clip.push(p));
        let triangle = vertices(&[(-2., 200.), (16., 190.), (16., 210.)]);
        for (label, f) in [
            (
                "reference",
                reference_clip_polygon as fn(Vec<ClipVertex>, &[(f32, f32)]) -> Vec<ClipVertex>,
            ),
            (
                "optimized",
                clip_polygon as fn(Vec<ClipVertex>, &[(f32, f32)]) -> Vec<ClipVertex>,
            ),
        ] {
            let start = std::time::Instant::now();
            for _ in 0..10_000 {
                std::hint::black_box(f(std::hint::black_box(triangle.clone()), &clip));
            }
            eprintln!("clip {label}: {:?}", start.elapsed());
        }
    }
}
