use viewkit::omochi::{
    DEFAULT_BURGERS, DEFAULT_MATERIAL, creep_compliance, loss_energy_ratio, normalized_creep_gain,
    recovery_envelope, recovery_strain, strain,
};

const EPSILON: f32 = 0.00001;

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, got {actual}",
    );
}

#[test]
fn creep_compliance_matches_playground_reference() {
    assert_close(creep_compliance(0.0, DEFAULT_BURGERS), 0.32258064);
    assert_close(creep_compliance(0.1, DEFAULT_BURGERS), 0.5267553);
    assert_close(creep_compliance(0.5, DEFAULT_BURGERS), 1.0453825);
    assert_close(creep_compliance(0.68, DEFAULT_BURGERS), 1.1791295);
}

#[test]
fn strain_matches_playground_reference() {
    assert_close(strain(0.5, 2.0, DEFAULT_BURGERS), 2.090765);
}

#[test]
fn recovery_strain_matches_playground_reference() {
    assert_close(recovery_strain(0.0, 0.5, 1.0, DEFAULT_BURGERS), 0.72280186);
    assert_close(recovery_strain(0.5, 0.5, 1.0, DEFAULT_BURGERS), 0.2884797);
    assert_close(recovery_strain(1.0, 0.5, 1.0, DEFAULT_BURGERS), 0.12870148);
}

#[test]
fn recovery_strain_converges_to_permanent_component() {
    assert_close(
        recovery_strain(100.0, 0.5, 1.0, DEFAULT_BURGERS),
        0.035714287,
    );
}

#[test]
fn loss_energy_ratio_matches_playground_reference() {
    assert_close(loss_energy_ratio(0.46), 0.41947082);
}

#[test]
fn normalized_creep_gain_matches_playground_reference() {
    assert_close(
        normalized_creep_gain(0.0, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        0.27357525,
    );
    assert_close(
        normalized_creep_gain(0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        0.8865714,
    );
    assert_close(
        normalized_creep_gain(0.68, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        1.0,
    );
}

#[test]
fn normalized_creep_gain_is_capped() {
    assert_close(
        normalized_creep_gain(1.0, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        1.12,
    );
    assert_close(
        normalized_creep_gain(10.0, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        1.12,
    );
}

#[test]
fn recovery_envelope_matches_playground_reference() {
    assert_close(
        recovery_envelope(0.0, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        1.0,
    );
    assert_close(
        recovery_envelope(0.1, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        0.6104521,
    );
    assert_close(
        recovery_envelope(0.5, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        0.31458634,
    );
    assert_close(
        recovery_envelope(1.0, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        0.1448159,
    );
}

#[test]
fn creep_compliance_increases_with_time() {
    let first = creep_compliance(0.1, DEFAULT_BURGERS);
    let second = creep_compliance(0.5, DEFAULT_BURGERS);
    let third = creep_compliance(1.0, DEFAULT_BURGERS);

    assert!(first < second);
    assert!(second < third);
}

#[test]
fn recovery_envelope_decreases_with_time() {
    let first = recovery_envelope(0.1, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS);
    let second = recovery_envelope(0.5, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS);
    let third = recovery_envelope(1.0, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS);

    assert!(first > second);
    assert!(second > third);
}

#[test]
fn negative_times_are_clamped_to_zero() {
    assert_close(
        creep_compliance(-10.0, DEFAULT_BURGERS),
        creep_compliance(0.0, DEFAULT_BURGERS),
    );
    assert_close(
        recovery_envelope(-10.0, 0.5, DEFAULT_MATERIAL, DEFAULT_BURGERS),
        1.0,
    );
}
