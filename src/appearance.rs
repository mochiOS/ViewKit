use crate::theme::{Color, Theme};

#[cfg(target_os = "mochios")]
const SETTINGS_PATH: &str = "/var/config/appearance/settings.conf";

const DEFAULT_FONT_SIZE: f32 = 13.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AppearanceSettings {
    appearance: usize,
    accent: usize,
    ui_scale: f32,
    font_size: f32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            appearance: 2,
            accent: 0,
            ui_scale: 1.0,
            font_size: DEFAULT_FONT_SIZE,
        }
    }
}

impl AppearanceSettings {
    pub(crate) fn load() -> Self {
        #[cfg(target_os = "mochios")]
        {
            return std::fs::read_to_string(SETTINGS_PATH)
                .ok()
                .map_or_else(Self::default, |text| Self::parse(&text));
        }

        #[cfg(not(target_os = "mochios"))]
        {
            Self::default()
        }
    }

    fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "appearance" => settings.appearance = parse_usize(value, 2, 2),
                "accent" => settings.accent = parse_usize(value, 0, 5),
                "ui_scale" => settings.ui_scale = parse_f32(value, 1.0, 0.75, 2.0),
                "font_size" => settings.font_size = parse_f32(value, DEFAULT_FONT_SIZE, 10.0, 24.0),
                _ => {}
            }
        }
        settings
    }

    pub(crate) fn theme(self) -> Theme {
        let theme = if self.appearance == 1 {
            Theme::DARK
        } else {
            Theme::LIGHT
        };
        theme.with_accent(accent_color(self.accent))
    }

    pub(crate) fn ui_scale(self) -> f64 {
        self.ui_scale as f64
    }

    pub(crate) fn font_scale(self) -> f32 {
        self.font_size / DEFAULT_FONT_SIZE
    }
}

/// Notifies running ViewKit applications that the persisted appearance changed.
///
/// The notification is authenticated by compositor.service using the caller's
/// `settings.write` capability. The settings file remains the source of truth.
pub fn notify_changed() -> bool {
    #[cfg(target_os = "mochios")]
    {
        crate::platform::mochios::notify_appearance_changed().is_ok()
    }

    #[cfg(not(target_os = "mochios"))]
    {
        true
    }
}

fn accent_color(accent: usize) -> Color {
    match accent {
        1 => Color::from_rgb_hex(0xaf52de),
        2 => Color::from_rgb_hex(0xff2d55),
        3 => Color::from_rgb_hex(0xff3b30),
        4 => Color::from_rgb_hex(0x34c759),
        5 => Color::from_rgb_hex(0x6e6e73),
        _ => Color::from_rgb_hex(0x0a84ff),
    }
}

fn parse_usize(value: &str, fallback: usize, maximum: usize) -> usize {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|value| *value <= maximum)
        .unwrap_or(fallback)
}

fn parse_f32(value: &str, fallback: f32, minimum: f32, maximum: f32) -> f32 {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(minimum, maximum))
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_persisted_appearance_values() {
        let settings =
            AppearanceSettings::parse("appearance=1\naccent=4\nui_scale=1.5\nfont_size=18\n");

        assert_eq!(settings.theme(), Theme::DARK.with_accent(accent_color(4)));
        assert_eq!(settings.ui_scale(), 1.5);
        assert_eq!(settings.font_scale(), 18.0 / DEFAULT_FONT_SIZE);
    }

    #[test]
    fn malformed_values_fall_back_or_clamp() {
        let settings =
            AppearanceSettings::parse("appearance=9\naccent=invalid\nui_scale=8\nfont_size=2\n");

        assert_eq!(settings.theme(), Theme::LIGHT.with_accent(accent_color(0)));
        assert_eq!(settings.ui_scale(), 2.0);
        assert_eq!(settings.font_scale(), 10.0 / DEFAULT_FONT_SIZE);
    }
}
