//! Native closed contours used by deformable omochi surfaces.
use crate::geometry::Point;

pub(crate) fn clean(points: &[Point]) -> Vec<Point> {
    if points.len() > 2048 || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Vec::new();
    }
    let mut result: Vec<Point> = Vec::with_capacity(points.len());
    for &p in points {
        if result
            .last()
            .is_none_or(|q| (p.x - q.x).hypot(p.y - q.y) > 0.001)
        {
            result.push(p);
        }
    }
    if result.len() > 1
        && (result[0].x - result.last().unwrap().x).hypot(result[0].y - result.last().unwrap().y)
            < 0.001
    {
        result.pop();
    }
    result
}
fn cross(a: Point, b: Point, c: Point) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
fn area(points: &[Point]) -> f32 {
    (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            a.x * b.y - a.y * b.x
        })
        .sum::<f32>()
        * 0.5
}

/// Ear clipping also supports the concave neck of a pulled surface.
pub(crate) fn triangulate(points: &[Point]) -> Vec<[usize; 3]> {
    let mut remaining: Vec<usize> = (0..points.len()).collect();
    let mut triangles = Vec::with_capacity(points.len().saturating_sub(2));
    let winding = area(points).signum();
    while remaining.len() > 2 {
        let mut ear = None;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            let turn = cross(points[a], points[b], points[c]) * winding;
            if turn.abs() < 0.000001 {
                ear = Some((i, None));
                break;
            }
            if turn < 0.0 {
                continue;
            }
            if remaining.iter().any(|&p| {
                p != a
                    && p != b
                    && p != c
                    && cross(points[a], points[b], points[p]) * winding >= -0.000001
                    && cross(points[b], points[c], points[p]) * winding >= -0.000001
                    && cross(points[c], points[a], points[p]) * winding >= -0.000001
            }) {
                continue;
            }
            ear = Some((i, Some([a, b, c])));
            break;
        }
        let Some((index, triangle)) = ear else {
            return Vec::new();
        };
        if let Some(triangle) = triangle {
            triangles.push(triangle);
        }
        remaining.remove(index);
    }
    triangles
}

pub(crate) fn offset(points: &[Point], distance: f32) -> Vec<Point> {
    let winding = area(points).signum();
    let normal = |a: Point, b: Point| {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx.hypot(dy).max(0.0001);
        Point::new(winding * dy / length, -winding * dx / length)
    };
    (0..points.len())
        .map(|i| {
            let p = points[i];
            let before = normal(points[(i + points.len() - 1) % points.len()], p);
            let after = normal(p, points[(i + 1) % points.len()]);
            let x = before.x + after.x;
            let y = before.y + after.y;
            let denominator = (x * after.x + y * after.y).max(0.25);
            Point::new(
                p.x + distance * x / denominator,
                p.y + distance * y / denominator,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangulates_concave_contour_without_filling_its_notch() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 4.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 4.0),
        ];
        let triangles = triangulate(&points);
        assert_eq!(triangles.len(), 3);
        let filled: f32 = triangles
            .iter()
            .map(|t| cross(points[t[0]], points[t[1]], points[t[2]]).abs() * 0.5)
            .sum();
        assert!((filled - area(&points).abs()).abs() < 0.001);
    }
}

/// Nonzero-winding tessellation. Folded surfaces can intersect themselves near
/// a press at the corner; SVG's winding rule must be preserved in that case.
pub(crate) fn tessellate(points: &[Point]) -> Vec<[Point; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let mut levels: Vec<f32> = points.iter().map(|p| p.y).collect();
    let mut intersects = false;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        for j in i + 2..n {
            if i == 0 && j + 1 == n {
                continue;
            }
            let c = points[j];
            let d = points[(j + 1) % n];
            if a.x.max(b.x) < c.x.min(d.x)
                || c.x.max(d.x) < a.x.min(b.x)
                || a.y.max(b.y) < c.y.min(d.y)
                || c.y.max(d.y) < a.y.min(b.y)
            {
                continue;
            }
            let rx = b.x - a.x;
            let ry = b.y - a.y;
            let sx = d.x - c.x;
            let sy = d.y - c.y;
            let denominator = rx * sy - ry * sx;
            if denominator.abs() < 0.000001 {
                continue;
            }
            let t = ((c.x - a.x) * sy - (c.y - a.y) * sx) / denominator;
            let u = ((c.x - a.x) * ry - (c.y - a.y) * rx) / denominator;
            if t > 0.000001 && t < 0.999999 && u > 0.000001 && u < 0.999999 {
                levels.push(a.y + t * ry);
                intersects = true;
            }
        }
    }
    if !intersects {
        let triangles = triangulate(points);
        if !triangles.is_empty() {
            return triangles
                .into_iter()
                .map(|t| t.map(|i| points[i]))
                .collect();
        }
    }
    levels.sort_by(f32::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() < 0.000001);
    let mut triangles = Vec::new();
    let at = |i: usize, y: f32| {
        let a = points[i];
        let b = points[(i + 1) % n];
        a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y)
    };
    for band in levels.windows(2) {
        let y0 = band[0];
        let y1 = band[1];
        let mid = (y0 + y1) * 0.5;
        if y1 - y0 < 0.000001 {
            continue;
        }
        let mut edges: Vec<(f32, usize, i32)> = (0..n)
            .filter_map(|i| {
                let a = points[i];
                let b = points[(i + 1) % n];
                (mid > a.y.min(b.y) && mid < a.y.max(b.y))
                    .then(|| (at(i, mid), i, if b.y > a.y { 1 } else { -1 }))
            })
            .collect();
        edges.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0;
        let mut left = 0;
        for (_, edge, sign) in edges {
            let before = winding;
            winding += sign;
            if before == 0 && winding != 0 {
                left = edge;
            }
            if before != 0 && winding == 0 {
                let a = Point::new(at(left, y0), y0);
                let b = Point::new(at(edge, y0), y0);
                let c = Point::new(at(edge, y1), y1);
                let d = Point::new(at(left, y1), y1);
                if cross(a, b, c).abs() > 0.000001 {
                    triangles.push([a, b, c]);
                }
                if cross(a, c, d).abs() > 0.000001 {
                    triangles.push([a, c, d]);
                }
            }
        }
    }
    triangles
}
