use super::material::{BurgersParameters, OmochiMaterial};

pub fn creep_compliance(time: f32, parameters: BurgersParameters) -> f32 {
    let time = time.max(0.0);
    let tau = parameters.eta2 / parameters.e2;

    1.0 / parameters.e1 + 1.0 / parameters.e2 * (1.0 - (-time / tau).exp()) + time / parameters.eta1
}

pub fn strain(time: f32, stress: f32, parameters: BurgersParameters) -> f32 {
    stress * creep_compliance(time, parameters)
}

pub fn recovery_strain(
    time_after_release: f32,
    load_duration: f32,
    stress: f32,
    parameters: BurgersParameters,
) -> f32 {
    let time = time_after_release.max(0.0);
    let loaded_for = load_duration.max(0.0);
    let tau = parameters.eta2 / parameters.e2;
    let permanent = stress * loaded_for / parameters.eta1;
    let delayed = stress / parameters.e2 * (1.0 - (-loaded_for / tau).exp());

    permanent + delayed * (-time / tau).exp()
}

pub fn loss_energy_ratio(tan_delta: f32) -> f32 {
    let value = tan_delta.max(0.0);

    std::f32::consts::PI * value / (2.0 + std::f32::consts::PI * value)
}

pub fn normalized_creep_gain(
    time: f32,
    material: OmochiMaterial,
    burgers: BurgersParameters,
) -> f32 {
    let reference = creep_compliance(material.reference_creep_time, burgers);

    (creep_compliance(time, burgers) / reference).clamp(0.0, 1.12)
}

pub fn recovery_envelope(
    time_after_release: f32,
    load_duration: f32,
    material: OmochiMaterial,
    burgers: BurgersParameters,
) -> f32 {
    let time = time_after_release.max(0.0);
    let loaded_for = load_duration.max(0.001);
    let tau = burgers.eta2 / burgers.e2;
    let elastic = 1.0 / burgers.e1;
    let delayed = 1.0 / burgers.e2 * (1.0 - (-loaded_for / tau).exp());
    let viscous = loaded_for / burgers.eta1;
    let total = elastic + delayed + viscous;
    let loss = loss_energy_ratio(material.tan_delta);
    let elastic_time = material.spring_recovery_time * (1.0 + loss * 0.35);
    let delayed_time = tau * (1.0 + loss * 0.55);
    let viscous_time = material.flow_recovery_time * (1.0 + loss * 0.80);

    elastic / total * (-time / elastic_time).exp()
        + delayed / total * (-time / delayed_time).exp()
        + viscous / total * (-time / viscous_time).exp()
}
