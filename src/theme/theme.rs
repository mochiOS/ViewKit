//! ViewKit全体の外観テーマを定義

use super::{
    Color, ColorTokens, DividerTokens, MotionTokens, RadiusTokens, ScrollBarTokens, ShadowTokens,
    SpacingTokens,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub shadows: ShadowTokens,
    pub divider: DividerTokens,
    pub scrollbar: ScrollBarTokens,
    pub motion: MotionTokens,
}

impl Theme {
    pub const LIGHT: Self = Self {
        colors: ColorTokens {
            background: Color::from_rgb_hex(0xf7f7f7),
            surface: Color::WHITE,
            surface_subtle: Color::from_rgb_hex(0xf2f2f2),
            surface_muted: Color::from_rgb_hex(0xe9e9e9),
            elevated_surface: Color::WHITE,

            text_primary: Color::from_rgb_hex(0x0a0a0a),
            text_secondary: Color::from_rgb_hex(0x606060),
            text_tertiary: Color::from_rgb_hex(0x8c8c8c),
            text_disabled: Color::from_rgb_hex(0x8c8c8c),

            accent: Color::from_rgb_hex(0x0a84ff),
            accent_hovered: Color::from_rgb_hex(0x0077e6),
            accent_pressed: Color::from_rgb_hex(0x006bc7),
            accent_soft: Color::rgba(10, 132, 255, 28),

            border: Color::rgba(0, 0, 0, 20),
            border_strong: Color::rgba(0, 0, 0, 38),
            focus_ring: Color::rgba(10, 132, 255, 71),

            success: Color::from_rgb_hex(0x218739),
            success_soft: Color::from_rgb_hex(0xe8f6eb),

            warning: Color::from_rgb_hex(0x8a5a00),
            warning_soft: Color::from_rgb_hex(0xfff4d7),

            destructive: Color::from_rgb_hex(0xc42b1c),
            destructive_hovered: Color::from_rgb_hex(0xe81123),
            destructive_soft: Color::from_rgb_hex(0xfff0ef),
        },

        radius: RadiusTokens::DEFAULT,
        spacing: SpacingTokens::DEFAULT,
        shadows: ShadowTokens::DEFAULT,
        divider: DividerTokens::DEFAULT,
        scrollbar: ScrollBarTokens::DEFAULT,
        motion: MotionTokens::DEFAULT,
    };
    pub const DEFAULT: Self = Theme::LIGHT;
}
