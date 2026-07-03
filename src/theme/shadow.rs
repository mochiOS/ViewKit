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
pub struct ShadowSet {
    pub layers: [Option<Shadow>; 2],
}

impl ShadowSet {
    pub const fn single(shadow: Shadow) -> Self {
        Self {
            layers: [Some(shadow), None],
        }
    }

    pub const fn double(first: Shadow, second: Shadow) -> Self {
        Self {
            layers: [Some(first), Some(second)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowTokens {
    pub card: ShadowSet,
    pub floating: ShadowSet,
    pub window: ShadowSet,
}

impl ShadowTokens {
    pub const DEFAULT: Self = Self {
        card: ShadowSet::single(Shadow::new(Color::rgba(0, 0, 0, 4), 0.0, 2.0, 8.0, 0.0)),

        floating: ShadowSet::double(
            Shadow::new(Color::rgba(0, 0, 0, 31), 0.0, 8.0, 24.0, 0.0),
            Shadow::new(Color::rgba(0, 0, 0, 31), 0.0, 24.0, 72.0, 0.0),
        ),

        window: ShadowSet::double(
            Shadow::new(Color::rgba(0, 0, 0, 20), 0.0, 2.0, 6.0, 0.0),
            Shadow::new(Color::rgba(0, 0, 0, 43), 0.0, 24.0, 72.0, 0.0),
        ),
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ShadowStyle {
    #[default]
    None,

    Card,
    Floating,
    Window,

    Custom(ShadowSet),
}

impl ShadowStyle {
    pub fn resolve(self, tokens: &ShadowTokens) -> Option<ShadowSet> {
        match self {
            Self::None => None,

            Self::Card => Some(tokens.card),

            Self::Floating => Some(tokens.floating),

            Self::Window => Some(tokens.window),

            Self::Custom(shadow_set) => Some(shadow_set),
        }
    }
}
