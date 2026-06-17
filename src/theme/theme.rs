use crate::theme::spacing::SpacingTokens;
use crate::theme::radius::RadiusTokens;
use super::{
    Color,
    ColorTokens,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
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
        radius: RadiusTokens::DEFAULT,
        spacing: SpacingTokens::DEFAULT,
    };
}