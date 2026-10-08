//! Shared elevation shadows for controls, surfaces, and windows.

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
        card: ShadowSet::double(
            Shadow::new(Color::rgba(0, 0, 0, 12), 0.0, 1.0, 2.0, 0.0),
            Shadow::new(Color::rgba(0, 0, 0, 5), 0.0, 2.0, 5.0, 0.0),
        ),

        floating: ShadowSet::double(
            Shadow::new(Color::rgba(0, 0, 0, 22), 0.0, 4.0, 12.0, 0.0),
            Shadow::new(Color::rgba(0, 0, 0, 12), 0.0, 10.0, 28.0, 0.0),
        ),

        window: ShadowSet::double(
            Shadow::new(Color::rgba(0, 0, 0, 16), 0.0, 2.0, 5.0, 0.0),
            Shadow::new(Color::rgba(0, 0, 0, 24), 0.0, 10.0, 28.0, 0.0),
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

#[cfg(test)]
mod tests {
    use super::ShadowTokens;

    #[test]
    fn shared_elevation_shadows_use_compact_blur_radii() {
        let tokens = ShadowTokens::DEFAULT;
        let card = tokens.card.layers.into_iter().flatten().collect::<Vec<_>>();
        let floating = tokens
            .floating
            .layers
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let window = tokens
            .window
            .layers
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert!(card.iter().all(|shadow| shadow.blur_radius <= 5.0));
        assert!(floating.iter().all(|shadow| shadow.blur_radius <= 28.0));
        assert!(window.iter().all(|shadow| shadow.blur_radius <= 28.0));
        assert!(floating.iter().all(|shadow| shadow.color.alpha <= 22));
        assert!(window.iter().all(|shadow| shadow.color.alpha <= 24));
    }
}
