#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpacingTokens {
    pub micro: f32,
    pub extra_small: f32,
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub extra_large: f32,
    pub double_extra_large: f32,
    pub triple_extra_large: f32,
    pub huge: f32,
    pub giant: f32,
}

impl SpacingTokens {
    pub const DEFAULT: Self = Self {
        micro: 2.0,
        extra_small: 4.0,
        small: 8.0,
        medium: 12.0,
        large: 16.0,
        extra_large: 24.0,
        double_extra_large: 32.0,
        triple_extra_large: 40.0,
        huge: 48.0,
        giant: 64.0,
    };
}
