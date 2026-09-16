#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiusTokens {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub extra_large: f32,
    pub card: f32,
    pub full: f32,
}

impl RadiusTokens {
    pub const DEFAULT: Self = Self {
        small: FigmaTokens::SHAPE.radius_small,
        medium: FigmaTokens::SHAPE.radius_medium,
        large: FigmaTokens::SHAPE.radius_large,
        extra_large: FigmaTokens::SHAPE.radius_xlarge,
        card: FigmaTokens::SHAPE.radius_large,
        full: 9999.0,
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CornerRadius {
    #[default]
    None,
    Small,
    Medium,
    Large,
    ExtraLarge,
    Card,
    Full,
    Custom(f32),
}

impl CornerRadius {
    pub fn resolve(self, tokens: &RadiusTokens, width: f32, height: f32) -> f32 {
        let value = match self {
            Self::None => 0.0,
            Self::Small => tokens.small,
            Self::Medium => tokens.medium,
            Self::Large => tokens.large,
            Self::ExtraLarge => tokens.extra_large,
            Self::Card => tokens.card,
            Self::Full => tokens.full,
            Self::Custom(value) => value,
        };

        value.max(0.0).min(width.min(height) / 2.0)
    }
}
use super::FigmaTokens;
