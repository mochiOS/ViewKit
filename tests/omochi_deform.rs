use viewkit::geometry::Point;
use viewkit::omochi::{DEFAULT_MATERIAL, clamp_vector, deform_points, gaussian, smoothstep};

const EPSILON: f32 = 0.0001;

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}",
    );
}

fn assert_point_close(actual: Point, expected: Point) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

fn rectangle() -> Vec<Point> {
    vec![
        Point::new(-48.0, -15.0),
        Point::new(48.0, -15.0),
        Point::new(48.0, 15.0),
        Point::new(-48.0, 15.0),
    ]
}

#[test]
fn clamp_vector_keeps_vectors_inside_limit() {
    assert_point_close(clamp_vector(3.0, 4.0, 10.0), Point::new(3.0, 4.0));
}

#[test]
fn clamp_vector_limits_vector_length() {
    assert_point_close(clamp_vector(3.0, 4.0, 2.5), Point::new(1.5, 2.0));
}

#[test]
fn gaussian_is_one_at_origin() {
    assert_close(gaussian(0.0, 20.0), 1.0);
}

#[test]
fn smoothstep_clamps_outside_range() {
    assert_close(smoothstep(-1.0), 0.0);
    assert_close(smoothstep(0.0), 0.0);
    assert_close(smoothstep(0.5), 0.5);
    assert_close(smoothstep(1.0), 1.0);
    assert_close(smoothstep(2.0), 1.0);
}

#[test]
fn deformation_without_pull_or_press_keeps_original_shape() {
    let points = rectangle();
    let deformed = deform_points(
        &points,
        Point::new(0.0, 0.0),
        0.0,
        Point::new(0.0, 0.0),
        DEFAULT_MATERIAL,
    );

    assert_eq!(deformed, points);
}

#[test]
fn centered_press_matches_playground_reference() {
    let deformed = deform_points(
        &rectangle(),
        Point::new(0.0, 0.0),
        9.5,
        Point::new(0.0, 0.0),
        DEFAULT_MATERIAL,
    );

    let expected = [
        Point::new(-40.19078, -12.55962),
        Point::new(40.19078, -12.55962),
        Point::new(40.19078, 12.55962),
        Point::new(-40.19078, 12.55962),
    ];

    for (actual, expected) in deformed.iter().zip(expected) {
        assert_point_close(*actual, expected);
    }
}

#[test]
fn right_pull_matches_playground_reference() {
    let deformed = deform_points(
        &rectangle(),
        Point::new(10.0, 0.0),
        0.0,
        Point::new(0.0, 0.0),
        DEFAULT_MATERIAL,
    );

    let expected = [
        Point::new(-49.57668, -15.0),
        Point::new(52.53189, -14.62240),
        Point::new(52.53189, 14.62240),
        Point::new(-49.57668, 15.0),
    ];

    for (actual, expected) in deformed.iter().zip(expected) {
        assert_point_close(*actual, expected);
    }
}

#[test]
fn right_edge_pull_matches_playground_reference() {
    let deformed = deform_points(
        &rectangle(),
        Point::new(10.0, 0.0),
        0.0,
        Point::new(48.0, 0.0),
        DEFAULT_MATERIAL,
    );

    let expected = [
        Point::new(-48.79409, -15.0),
        Point::new(51.74930, -14.62240),
        Point::new(51.74930, 14.62240),
        Point::new(-48.79409, 15.0),
    ];

    for (actual, expected) in deformed.iter().zip(expected) {
        assert_point_close(*actual, expected);
    }
}

#[test]
fn left_pull_is_horizontal_mirror_of_right_pull() {
    let right = deform_points(
        &rectangle(),
        Point::new(10.0, 0.0),
        0.0,
        Point::new(0.0, 0.0),
        DEFAULT_MATERIAL,
    );

    let left = deform_points(
        &rectangle(),
        Point::new(-10.0, 0.0),
        0.0,
        Point::new(0.0, 0.0),
        DEFAULT_MATERIAL,
    );

    assert_point_close(left[0], Point::new(-right[1].x, right[1].y));
    assert_point_close(left[1], Point::new(-right[0].x, right[0].y));
    assert_point_close(left[2], Point::new(-right[3].x, right[3].y));
    assert_point_close(left[3], Point::new(-right[2].x, right[2].y));
}

#[test]
fn pulled_tip_extends_outward() {
    let deformed = deform_points(
        &rectangle(),
        Point::new(10.0, 0.0),
        0.0,
        Point::new(48.0, 0.0),
        DEFAULT_MATERIAL,
    );

    assert!(deformed[1].x > 48.0);
    assert!(deformed[2].x > 48.0);
}

#[test]
fn deformation_never_produces_non_finite_points() {
    let deformed = deform_points(
        &rectangle(),
        Point::new(46.0, -46.0),
        9.5,
        Point::new(48.0, -15.0),
        DEFAULT_MATERIAL,
    );

    for point in deformed {
        assert!(point.x.is_finite());
        assert!(point.y.is_finite());
    }
}

#[test]
fn empty_shape_stays_empty() {
    let deformed = deform_points(
        &[],
        Point::new(10.0, 0.0),
        9.5,
        Point::new(0.0, 0.0),
        DEFAULT_MATERIAL,
    );

    assert!(deformed.is_empty());
}
