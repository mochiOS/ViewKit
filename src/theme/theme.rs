//! ViewKit全体の外観テーマを定義

use super::{
    BrowserTokens, Color, ColorTokens, DividerTokens, MotionTokens, RadiusTokens, ScrollBarTokens,
    ShadowTokens, ShellTokens, SpacingTokens,
};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub shadows: ShadowTokens,
    pub divider: DividerTokens,
    pub scrollbar: ScrollBarTokens,
    pub motion: MotionTokens,
    pub shell: ShellTokens,
    pub browser: BrowserTokens,
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
            accent_soft: Color::rgba(200, 200, 200, 25),

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
        shell: ShellTokens::LIGHT,
        browser: BrowserTokens::LIGHT,
    };
    pub const DARK: Self = Self {
        colors: ColorTokens {
            background: Color::from_rgb_hex(0x1c1c1e),
            surface: Color::from_rgb_hex(0x242426),
            surface_subtle: Color::from_rgb_hex(0x2c2c2e),
            surface_muted: Color::from_rgb_hex(0x3a3a3c),
            elevated_surface: Color::from_rgb_hex(0x323234),

            text_primary: Color::from_rgb_hex(0xf5f5f7),
            text_secondary: Color::from_rgb_hex(0xaeaeb2),
            text_tertiary: Color::from_rgb_hex(0x8e8e93),
            text_disabled: Color::from_rgb_hex(0x636366),

            accent: Color::from_rgb_hex(0x0a84ff),
            accent_hovered: Color::from_rgb_hex(0x409cff),
            accent_pressed: Color::from_rgb_hex(0x0071db),
            accent_soft: Color::rgba(10, 132, 255, 38),

            border: Color::rgba(255, 255, 255, 26),
            border_strong: Color::rgba(255, 255, 255, 46),
            focus_ring: Color::rgba(10, 132, 255, 102),

            success: Color::from_rgb_hex(0x32d74b),
            success_soft: Color::from_rgb_hex(0x17351e),

            warning: Color::from_rgb_hex(0xffd60a),
            warning_soft: Color::from_rgb_hex(0x3d3308),

            destructive: Color::from_rgb_hex(0xff453a),
            destructive_hovered: Color::from_rgb_hex(0xff6961),
            destructive_soft: Color::from_rgb_hex(0x3d1715),
        },

        radius: RadiusTokens::DEFAULT,
        spacing: SpacingTokens::DEFAULT,
        shadows: ShadowTokens::DEFAULT,
        divider: DividerTokens::DEFAULT,
        scrollbar: ScrollBarTokens::DEFAULT,
        motion: MotionTokens::DEFAULT,
        shell: ShellTokens::DARK,
        browser: BrowserTokens::DARK,
    };
    pub const DEFAULT: Self = Theme::LIGHT;

    #[must_use]
    pub fn with_accent(mut self, accent: Color) -> Self {
        let dark = self.colors.background == Self::DARK.colors.background;
        self.colors.accent = accent;
        self.colors.accent_hovered = mix(accent, Color::WHITE, 36);
        self.colors.accent_pressed = mix(accent, Color::BLACK, 28);
        self.colors.accent_soft = accent.with_alpha(if dark { 38 } else { 25 });
        self.colors.focus_ring = accent.with_alpha(if dark { 102 } else { 71 });
        self.shell.action = accent;
        self.shell.action_hover = self.colors.accent_hovered;
        self.shell.action_pressed = self.colors.accent_pressed;
        self.shell.selection_soft = accent.with_alpha(if dark { 48 } else { 28 });
        self.shell.selection_border = accent.with_alpha(if dark { 112 } else { 72 });
        self.browser.selection = accent;
        self
    }

    /// Returns the theme currently used while constructing a View tree.
    #[must_use]
    pub fn current() -> Self {
        CURRENT_THEME.with(Cell::get)
    }

    pub(crate) fn set_current(theme: Self) {
        CURRENT_THEME.with(|current| current.set(theme));
    }
}

thread_local! {
    static CURRENT_THEME: Cell<Theme> = const { Cell::new(Theme::DEFAULT) };
}

fn mix(base: Color, other: Color, other_weight: u8) -> Color {
    let weight = u16::from(other_weight.min(100));
    let base_weight = 100 - weight;
    let channel = |base: u8, other: u8| {
        ((u16::from(base) * base_weight + u16::from(other) * weight) / 100) as u8
    };
    Color::rgba(
        channel(base.red, other.red),
        channel(base.green, other.green),
        channel(base.blue, other.blue),
        base.alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_is_applied_to_application_tokens() {
        let accent = Color::from_rgb_hex(0x7651c9);
        let theme = Theme::LIGHT.with_accent(accent);

        assert_eq!(theme.colors.accent, accent);
        assert_eq!(theme.shell.action, accent);
        assert_eq!(theme.shell.selection_soft, accent.with_alpha(28));
        assert_eq!(theme.shell.selection_border, accent.with_alpha(72));
        assert_eq!(theme.browser.selection, accent);
    }

    #[test]
    fn dark_accent_uses_dark_selection_opacity() {
        let accent = Color::from_rgb_hex(0x7651c9);
        let theme = Theme::DARK.with_accent(accent);

        assert_eq!(theme.shell.selection_soft, accent.with_alpha(48));
        assert_eq!(theme.shell.selection_border, accent.with_alpha(112));
    }
}
