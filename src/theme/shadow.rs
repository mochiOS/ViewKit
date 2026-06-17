//! 影！！！影は薄くねえぞ！！！

use super::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub color: Color,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread: f32,
}

impl Shadow {
    pub const fn new(
        color: Color,
        offset_x: f32,
        offset_y: f32,
        blur_radius: f32,
        spread: f32,
    ) -> Self {
        Self {
            color,
            offset_x,
            offset_y,
            blur_radius,
            spread,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowTokens {
    pub small: Shadow,
    pub medium: Shadow,
    pub large: Shadow,
}

impl ShadowTokens {
    pub const DEFAULT: Self = Self {
        small: Shadow::new(
            Color::rgba(0, 0, 0, 32),
            0.0,
            2.0,
            6.0,
            0.0,
        ),

        medium: Shadow::new(
            Color::rgba(0, 0, 0, 40),
            0.0,
            4.0,
            12.0,
            0.0,
        ),

        large: Shadow::new(
            Color::rgba(0, 0, 0, 48),
            0.0,
            8.0,
            24.0,
            0.0,
        ),
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ShadowStyle {
    #[default]
    None,
    Small,
    Medium,
    Large,
    Custom(Shadow),
}

impl ShadowStyle {
    pub fn resolve(
        self,
        tokens: &ShadowTokens,
    ) -> Option<Shadow> {
        match self {
            Self::None => None,
            Self::Small => Some(tokens.small),
            Self::Medium => Some(tokens.medium),
            Self::Large => Some(tokens.large),
            Self::Custom(shadow) => Some(shadow),
        }
    }
}