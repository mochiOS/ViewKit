//! mochi design language
//!
//! see it: (todo)

mod burgers;
mod deform;
mod material;

pub use burgers::{
    creep_compliance, loss_energy_ratio, normalized_creep_gain, recovery_envelope, recovery_strain,
    strain,
};
pub use deform::{clamp_vector, deform_points, gaussian, smoothstep};
pub use material::{BurgersParameters, DEFAULT_BURGERS, DEFAULT_MATERIAL, OmochiMaterial};
