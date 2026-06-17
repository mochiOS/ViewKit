use crate::theme::color::{Color, ColorTokens};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpacingTokens {
    pub extra_small: u16,
    pub small: u16,
    pub medium: u16,
    pub large: u16,
    pub extra_large: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiusTokens {
    pub small: f32,
    pub medium: f32,
    pub large: f32,
    pub full: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub spacing: SpacingTokens,
    pub radius: RadiusTokens,
}

impl Theme {
    pub const LIGHT: Self = Self {
        colors: ColorTokens {
            background: Color::from_rgb_hex(0xf5f5f7),
            surface: Color::WHITE,
            elevated_surface: Color::WHITE,

            text_primary: Color::from_rgb_hex(0x1d1d1f),
            text_secondary: Color::from_rgb_hex(0x6e6e73),
            text_disabled: Color::from_rgb_hex(0xaeaeb2),

            accent: Color::from_rgb_hex(0x007aff),
            accent_hovered: Color::from_rgb_hex(0x006ee6),
            accent_pressed: Color::from_rgb_hex(0x005fc7),

            border: Color::from_rgb_hex(0xd2d2d7),
            focus_ring: Color::from_rgb_hex(0x007aff),
            destructive: Color::from_rgb_hex(0xff3b30),
        },

        spacing: SpacingTokens {
            extra_small: 4,
            small: 8,
            medium: 12,
            large: 16,
            extra_large: 24,
        },

        radius: RadiusTokens {
            small: 6.0,
            medium: 10.0,
            large: 16.0,
            full: 9999.0,
        },
    };
}