//! mochi design language
//!
//! see it: (todo)

mod burgers;
mod material;

pub use burgers::{
    creep_compliance, loss_energy_ratio, normalized_creep_gain, recovery_envelope, recovery_strain,
    strain,
};
pub use material::{BurgersParameters, DEFAULT_BURGERS, DEFAULT_MATERIAL, OmochiMaterial};
