#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiusTokens {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub extra_large: f32,
    pub full: f32,
}

impl RadiusTokens {
    pub const DEFAULT: Self = Self {
        small: 6.0,
        medium: 10.0,
        large: 14.0,
        extra_large: 20.0,
        full: 9999.0,
    };
}