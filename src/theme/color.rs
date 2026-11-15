//! 色を定義

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
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
    /// Relative luminance of opaque sRGB channels, used for contrast decisions.
    /// Translucent colors must first be composited over their actual background.
    pub fn relative_luminance(self) -> f32 {
        let linear = |c: u8| {
            let v = c as f32 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.red) + 0.7152 * linear(self.green) + 0.0722 * linear(self.blue)
    }
    /// WCAG luminance contrast for two already-composited colors.
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let a = self.relative_luminance();
        let b = other.relative_luminance();
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    /// Chooses opaque black or white with the greater luminance contrast.
    pub fn readable_foreground(self) -> Self {
        if self.contrast_ratio(Self::BLACK) >= self.contrast_ratio(Self::WHITE) {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }
    /// Composites unassociated UI colors under the encoded-UNORM surface contract.
    /// This is also used to predict the actual contrast of translucent materials.
    pub fn over(self, background: Self) -> Self {
        let premul = |c: Self| {
            [c.red, c.green, c.blue, c.alpha]
                .map(|v| ((v as u32 * c.alpha as u32 + 127) / 255) as u8)
        };
        let mut source = premul(self);
        source[3] = self.alpha;
        let mut target = premul(background);
        target[3] = background.alpha;
        let result = mochios_viewkit_gpu_protocol::raster_quality::source_over(target, source, 255);
        if result[3] == 0 {
            return Self::TRANSPARENT;
        }
        let channel =
            |v: u8| ((v as u32 * 255 + result[3] as u32 / 2) / result[3] as u32).min(255) as u8;
        Self::rgba(
            channel(result[0]),
            channel(result[1]),
            channel(result[2]),
            result[3],
        )
    }

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

#[cfg(test)]
mod contrast_tests {
    use super::*;
    #[test]
    fn adaptive_text_is_readable_across_the_neutral_range() {
        for v in 0..=255 {
            let bg = Color::rgb(v, v, v);
            assert!(bg.contrast_ratio(bg.readable_foreground()) >= 4.5);
        }
        assert_eq!(
            Color::rgba(255, 255, 255, 0).over(Color::BLACK),
            Color::BLACK
        );
        assert_eq!(
            Color::rgba(0, 0, 0, 128).over(Color::WHITE),
            Color::rgb(127, 127, 127)
        );
    }
}
