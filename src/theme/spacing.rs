#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpacingTokens {
    pub extra_small: f32,
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub extra_large: f32,
    pub double_extra_large: f32,
}

impl SpacingTokens {
    pub const DEFAULT: Self = Self {
        extra_small: 4.0,
        small: 8.0,
        medium: 12.0,
        large: 16.0,
        extra_large: 24.0,
        double_extra_large: 32.0,
    };
}
