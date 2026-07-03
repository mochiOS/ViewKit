//! 色を定義

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTokens {
    pub background: Color,

    pub surface: Color,
    pub surface_subtle: Color,
    pub surface_muted: Color,
    pub elevated_surface: Color,

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_disabled: Color,

    pub accent: Color,
    pub accent_hovered: Color,
    pub accent_pressed: Color,
    pub accent_soft: Color,

    pub border: Color,
    pub border_strong: Color,
    pub focus_ring: Color,

    pub success: Color,
    pub success_soft: Color,

    pub warning: Color,
    pub warning_soft: Color,

    pub destructive: Color,
    pub destructive_hovered: Color,
    pub destructive_soft: Color,
}

impl Color {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn from_rgb_hex(value: u32) -> Self {
        Self::rgb(
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )
    }

    pub const fn from_rgba_hex(value: u32) -> Self {
        Self::rgba(
            ((value >> 24) & 0xff) as u8,
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )
    }

    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self::rgba(self.red, self.green, self.blue, alpha)
    }

    pub fn alpha(self, opacity: f32) -> Self {
        let opacity = if opacity.is_nan() {
            0.0
        } else {
            opacity.clamp(0.0, 1.0)
        };

        self.with_alpha((opacity * 255.0).round() as u8)
    }

    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
}
