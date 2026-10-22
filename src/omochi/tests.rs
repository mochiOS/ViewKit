// Source SHA-256: d053b571f9a7917b03bbff7b46c2f861d31f9f86c73ff263e1b2c7ef63565165
// Numeric fixtures evaluated from ../omochi/playground/omochi.js using Node.
use super::*;
#[test]
fn matches_javascript_material_and_deformation_fixtures() {
    let motion = OmochiMotion::DEFAULT;
    assert!((creep_compliance(0.0, motion.burgers) - 0.322580645).abs() < 0.00001);
    assert!((normalized_creep_gain(0.0, motion) - 0.273575250).abs() < 0.00001);
    assert!((creep_compliance(0.05, motion.burgers) - 0.429589663).abs() < 0.00001);
    assert!((normalized_creep_gain(0.05, motion) - 0.364327808).abs() < 0.00001);
    assert!((creep_compliance(0.5, motion.burgers) - 1.045382495).abs() < 0.00001);
    assert!((normalized_creep_gain(0.5, motion) - 0.886571410).abs() < 0.00001);
    assert!((creep_compliance(2.0, motion.burgers) - 1.532486007).abs() < 0.00001);
    assert!((normalized_creep_gain(2.0, motion) - 1.120000000).abs() < 0.00001);
    assert!((recovery_envelope(0.01, 0.8, motion) - 0.926733804).abs() < 0.00001);
    assert!((recovery_envelope(0.25, 0.8, motion) - 0.501152178).abs() < 0.00001);
    assert!((recovery_envelope(1.0, 0.8, motion) - 0.157608132).abs() < 0.00001);
    assert!((recovery_envelope(3.0, 0.8, motion) - 0.009509082).abs() < 0.00001);
    let base = [
        Point::new(9.000000000, 0.000000000),
        Point::new(48.000000000, 0.000000000),
        Point::new(87.000000000, 0.000000000),
        Point::new(96.000000000, 9.000000000),
        Point::new(96.000000000, 21.000000000),
        Point::new(87.000000000, 30.000000000),
        Point::new(48.000000000, 30.000000000),
        Point::new(9.000000000, 30.000000000),
        Point::new(0.000000000, 21.000000000),
        Point::new(0.000000000, 9.000000000),
    ];
    {
        let sample = SurfaceSample {
            anchor: Point::new(48.000000000, 15.000000000),
            pull: Point::new(0.000000000, 0.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(10.130453815, 0.434789929),
            Point::new(48.000000000, 2.434998128),
            Point::new(85.869546185, 0.434789929),
            Point::new(95.085045083, 9.114369365),
            Point::new(95.085045083, 20.885630635),
            Point::new(85.869546185, 29.565210071),
            Point::new(48.000000000, 27.565001872),
            Point::new(10.130453815, 29.565210071),
            Point::new(0.914954917, 20.885630635),
            Point::new(0.914954917, 9.114369365),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 0: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(48.000000000, 15.000000000),
            pull: Point::new(12.000000000, 5.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(7.234214337, -0.735744127),
            Point::new(47.632352537, -0.144505711),
            Point::new(88.934917949, 1.436566051),
            Point::new(99.179961471, 10.858526161),
            Point::new(100.531707462, 23.167142861),
            Point::new(92.027970702, 32.148472805),
            Point::new(48.736093031, 30.253131433),
            Point::new(7.361165742, 29.317106904),
            Point::new(-1.769170152, 20.262844830),
            Point::new(-1.791570870, 8.253512023),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 1: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(48.000000000, 15.000000000),
            pull: Point::new(-8.000000000, -10.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(6.621283453, -3.442169135),
            Point::new(46.759252949, -1.376782397),
            Point::new(87.736271425, 0.937754139),
            Point::new(97.088183484, 10.361154957),
            Point::new(97.173086872, 22.466410954),
            Point::new(88.178757917, 31.473470836),
            Point::new(48.652811903, 30.809024607),
            Point::new(8.491004370, 28.802620272),
            Point::new(-0.760028704, 19.088972689),
            Point::new(-1.388587397, 6.552272097),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 2: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(10.000000000, 8.000000000),
            pull: Point::new(0.000000000, 0.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(9.325046914, 2.600375314),
            Point::new(46.677941714, 0.278328060),
            Point::new(86.828594551, 0.017808358),
            Point::new(95.909558397, 8.998948353),
            Point::new(95.917207252, 20.987484817),
            Point::new(86.863363133, 29.960960895),
            Point::new(47.035839109, 29.441801590),
            Point::new(9.098125890, 27.841230417),
            Point::new(1.454952670, 19.108561529),
            Point::new(2.564859724, 8.743514028),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 3: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(10.000000000, 8.000000000),
            pull: Point::new(12.000000000, 5.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(7.990206422, -0.420746925),
            Point::new(48.852809286, 0.367679256),
            Point::new(87.539984344, 0.912042330),
            Point::new(97.217256482, 10.173544601),
            Point::new(98.444359751, 22.423015161),
            Point::new(90.378392671, 31.669072065),
            Point::new(48.904923838, 30.349789264),
            Point::new(8.984861595, 29.993656993),
            Point::new(-1.056482916, 20.559798100),
            Point::new(-1.438441760, 8.400649213),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 4: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(10.000000000, 8.000000000),
            pull: Point::new(-8.000000000, -10.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(5.270764982, -4.523586118),
            Point::new(47.920673858, 0.371559799),
            Point::new(87.506562593, 0.652907565),
            Point::new(96.530741795, 9.664447284),
            Point::new(96.534311574, 21.667956682),
            Point::new(87.534543761, 30.668222377),
            Point::new(48.505106757, 30.641845941),
            Point::new(9.022584919, 29.767529361),
            Point::new(-1.486662432, 18.675188058),
            Point::new(-3.288627037, 4.640954042),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 5: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(90.000000000, 25.000000000),
            pull: Point::new(0.000000000, 0.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(9.095216573, 0.029387831),
            Point::new(48.774706096, 0.461134581),
            Point::new(87.240447584, 2.003729863),
            Point::new(95.170915842, 11.210891089),
            Point::new(93.806465284, 22.462356477),
            Point::new(88.367621565, 27.720630724),
            Point::new(49.179180344, 29.859621388),
            Point::new(9.131004328, 29.991913313),
            Point::new(0.064953129, 21.002886806),
            Point::new(0.057334462, 9.010192793),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 6: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(90.000000000, 25.000000000),
            pull: Point::new(12.000000000, 5.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(8.264466646, -0.306472675),
            Point::new(47.296861019, -0.287848635),
            Point::new(87.820175767, 0.853422036),
            Point::new(99.455561648, 10.851762657),
            Point::new(101.077637822, 23.271370117),
            Point::new(91.475897988, 31.683218006),
            Point::new(47.401957659, 29.677906312),
            Point::new(8.266525488, 29.694336040),
            Point::new(-0.735566085, 20.693512992),
            Point::new(-0.735915094, 8.693368552),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 7: {actual:?} != {expected:?}"
            );
        }
    }
    {
        let sample = SurfaceSample {
            anchor: Point::new(90.000000000, 25.000000000),
            pull: Point::new(-8.000000000, -10.000000000),
            press_depth: 2.7,
            next_redraw_at: None,
        };
        let actual = deform_points(&base, Point::new(48.0, 15.0), sample, motion.material);
        let expected = [
            Point::new(9.135292879, -0.690929538),
            Point::new(47.540954737, -0.849858328),
            Point::new(86.577296802, -0.520822723),
            Point::new(96.026292646, 9.033288070),
            Point::new(96.648436857, 21.810562235),
            Point::new(87.751882511, 30.939844737),
            Point::new(47.734608949, 29.648851779),
            Point::new(9.198541539, 29.613271375),
            Point::new(0.431779749, 20.480683906),
            Point::new(0.334261952, 8.503917444),
        ];
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual.x - expected.x).abs() < 0.00002 && (actual.y - expected.y).abs() < 0.00002,
                "fixture 8: {actual:?} != {expected:?}"
            );
        }
    }
}

#[test]
fn held_time_changes_recovery_and_short_taps_are_not_lost() {
    let motion = OmochiMotion::DEFAULT;
    let now = Instant::now();
    let mut surface = OmochiSurface::default();
    surface.begin(Point::new(10.0, 8.0), now);
    surface.end(motion, now + Duration::from_millis(5));
    let release = surface.sample(motion, now + Duration::from_millis(5));
    assert!(release.press_depth > 0.0);
    assert!(release.next_redraw_at.is_some());
    assert!(recovery_envelope(0.4, 2.0, motion) > recovery_envelope(0.4, 0.01, motion));
    let finished = surface.sample(motion, now + Duration::from_secs(15));
    assert_eq!(finished.press_depth, 0.0);
    assert!(finished.next_redraw_at.is_none());
    surface.begin(Point::new(48.0, 15.0), now);
    let reduced = OmochiMotion {
        enabled: false,
        ..motion
    };
    assert_eq!(surface.sample(reduced, now), SurfaceSample::default());
    assert!(!surface.is_active());
}

#[test]
fn idle_held_surface_stops_scheduling_until_pointer_moves() {
    let motion = OmochiMotion::DEFAULT;
    let now = Instant::now();
    let mut surface = OmochiSurface::default();
    surface.begin(Point::new(48.0, 15.0), now);
    assert!(surface.sample(motion, now).next_redraw_at.is_some());
    assert!(
        surface
            .sample(motion, now + Duration::from_secs(2))
            .next_redraw_at
            .is_none()
    );
    surface.move_to(Point::new(80.0, 20.0), now + Duration::from_secs(3));
    assert!(
        surface
            .sample(motion, now + Duration::from_secs(3))
            .next_redraw_at
            .is_some()
    );
}

#[test]
fn deformed_button_contours_are_finite_and_tessellate_including_the_neck() {
    let bounds = Rect::new(0.0, 0.0, 96.0, 30.0);
    let material = Material::BUTTON;
    for anchor in [
        Point::new(48.0, 15.0),
        Point::new(1.0, 1.0),
        Point::new(95.0, 29.0),
    ] {
        for pull in [
            Point::new(0.0, 0.0),
            Point::new(16.0, 0.0),
            Point::new(-16.0, 0.0),
            Point::new(0.0, 16.0),
            Point::new(0.0, -16.0),
            Point::new(10.0, 10.0),
        ] {
            let points = button_contour(
                bounds,
                9.0,
                SurfaceSample {
                    anchor,
                    pull,
                    press_depth: 3.8,
                    next_redraw_at: None,
                },
                material,
            );
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
            assert!(points.len() < 512);
            assert!(
                !crate::contour::tessellate(&points).is_empty(),
                "anchor={anchor:?}, pull={pull:?}"
            );
            let inner = crate::contour::offset(&points, -0.5);
            assert!(
                !crate::contour::tessellate(&inner).is_empty(),
                "inner: anchor={anchor:?}, pull={pull:?}"
            );
        }
    }
}

#[test]
fn slider_velocity_driver_matches_javascript_reference() {
    // Evaluated from ../omochi/playground/omochi.js with slider/index.html's material.
    let motion = OmochiMotion {
        material: Material::SLIDER,
        ..OmochiMotion::DEFAULT
    };
    let now = Instant::now();
    let mut surface = OmochiSurface::default();
    surface.begin_velocity(Point::default(), Point::new(-1.0, -0.12), now);
    for (ms, point, expected) in [
        (
            16,
            Some(Point::new(100.0, 8.0)),
            Point::new(-2.612470002, -0.025079712),
        ),
        (
            32,
            Some(Point::new(170.0, 12.0)),
            Point::new(-5.252563250, -0.045976583),
        ),
        (
            48,
            Some(Point::new(155.0, 15.0)),
            Point::new(-6.600629898, -0.075201546),
        ),
        (64, None, Point::new(-7.264491354, -0.091739859)),
        (96, None, Point::new(-7.107809970, -0.097667515)),
        (128, None, Point::new(-6.211244241, -0.088599120)),
        (160, None, Point::new(-5.094839100, -0.074148193)),
        (192, None, Point::new(-4.012711179, -0.059102497)),
    ] {
        let time = now + Duration::from_millis(ms);
        if let Some(point) = point {
            surface.move_to(point, time);
        }
        let sample = surface.sample(motion, time);
        assert!(
            (sample.pull.x - expected.x).abs() < 0.00002,
            "{ms}: {:?}",
            sample.pull
        );
        assert!((sample.pull.y - expected.y).abs() < 0.00002);
        assert_eq!(sample.press_depth, 0.0);
    }
    let mut settled = SurfaceSample::default();
    for ms in (208..8000).step_by(16) {
        settled = surface.sample(motion, now + Duration::from_millis(ms));
    }
    assert!(settled.next_redraw_at.is_none());
}
