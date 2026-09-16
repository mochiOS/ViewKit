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
        micro: FigmaTokens::SPACING.space_2,
        extra_small: FigmaTokens::SPACING.space_4,
        small: FigmaTokens::SPACING.space_8,
        medium: FigmaTokens::SPACING.space_12,
        large: FigmaTokens::SPACING.space_16,
        extra_large: FigmaTokens::SPACING.space_24,
        double_extra_large: FigmaTokens::SPACING.space_32,
        triple_extra_large: FigmaTokens::SPACING.space_40,
        huge: FigmaTokens::SPACING.space_48,
        giant: FigmaTokens::SPACING.space_64,
    };
}
use super::FigmaTokens;
