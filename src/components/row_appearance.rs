//! Selection and hover colors shared by list and navigation rows.
use super::ButtonStyle;
use crate::theme::{Color, Theme};

#[derive(Clone, Copy)]
pub(super) struct RowAppearance {
    pub foreground: Color,
    pub secondary: Color,
    pub leading: Color,
    pub background: Color,
    pub hovered_background: Color,
}
impl RowAppearance {
    pub fn resolve(theme: &Theme, selected: bool, enabled: bool, active: bool) -> Self {
        let tokens = &theme.list;
        let foreground = if !enabled {
            tokens.disabled_foreground
        } else if selected && active {
            tokens.selected_foreground
        } else if selected {
            tokens.inactive_selected_foreground
        } else {
            tokens.foreground
        };
        let background = if selected {
            if active && enabled {
                tokens.selected_background
            } else {
                tokens.inactive_selected_background
            }
        } else {
            tokens.background
        };
        Self {
            foreground,
            secondary: if enabled {
                tokens.secondary_foreground
            } else {
                tokens.disabled_foreground
            },
            leading: if !enabled {
                tokens.disabled_foreground
            } else if selected {
                foreground
            } else {
                tokens.leading_foreground
            },
            background,
            hovered_background: if selected || !active || !enabled {
                background
            } else {
                tokens.hovered_background
            },
        }
    }
    pub fn button_style(self) -> ButtonStyle {
        ButtonStyle::Custom {
            background: self.background,
            hovered_background: self.hovered_background,
            border: Color::TRANSPARENT,
            hovered_border: Color::TRANSPARENT,
            foreground: self.foreground,
        }
    }
}
