//! ViewKit全体の外観テーマを定義

use super::{Color, ColorTokens, RadiusTokens, ShadowTokens, SpacingTokens};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub shadows: ShadowTokens,
}

impl Theme {
    pub const LIGHT: Self = Self {
        colors: ColorTokens {
            background: Color::from_rgb_hex(0xfcfcfc),
            surface: Color::WHITE,
            elevated_surface: Color::WHITE,

            text_primary: Color::from_rgb_hex(0x1d1d1d),
            text_secondary: Color::from_rgb_hex(0x6e6e6e),
            text_disabled: Color::from_rgb_hex(0xaeaeae),

            accent: Color::from_rgb_hex(0x007aff),
            accent_hovered: Color::from_rgb_hex(0x006ee6),
            accent_pressed: Color::from_rgb_hex(0x005fc7),

            border: Color::from_rgb_hex(0xd2d2d2),
            focus_ring: Color::from_rgb_hex(0x007aff),
            destructive: Color::from_rgb_hex(0xff3b30),
        },
        radius: RadiusTokens::DEFAULT,
        spacing: SpacingTokens::DEFAULT,
        shadows: ShadowTokens::DEFAULT,
    };
    pub const DEFAULT: Self = Theme::LIGHT;
}