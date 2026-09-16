use super::{Color, ColorTokens, CornerRadius, FigmaTokens, Shadow, ShadowSet, ShadowStyle};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlVisualState {
    #[default]
    Rest,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlAppearance {
    pub background: Color,
    pub border: Color,
    pub foreground: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonPalette {
    pub rest: ControlAppearance,
    pub hovered: ControlAppearance,
    pub pressed: ControlAppearance,
}

impl ButtonPalette {
    pub const fn resolve(self, state: ControlVisualState) -> ControlAppearance {
        match state {
            ControlVisualState::Pressed => self.pressed,
            ControlVisualState::Hovered => self.hovered,
            ControlVisualState::Rest
            | ControlVisualState::Focused
            | ControlVisualState::Disabled => self.rest,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonTokens {
    pub standard: ButtonPalette,
    pub primary: ButtonPalette,
    pub accent: ButtonPalette,
    pub ghost: ButtonPalette,
    pub danger: ButtonPalette,
    pub horizontal_padding: f32,
    pub height: f32,
    pub radius: CornerRadius,
    pub stroke_width: f32,
    pub focus_ring: Color,
    pub focus_ring_width: f32,
    pub disabled_opacity: f32,
}

impl ButtonTokens {
    pub const fn from_colors(colors: ColorTokens, dark: bool) -> Self {
        let transparent = Color::TRANSPARENT;
        let primary_rest = if dark {
            Color::from_rgb_hex(0xf3f4f5)
        } else {
            Color::from_rgb_hex(0x17181a)
        };
        let primary_hover = if dark {
            Color::from_rgb_hex(0xffffff)
        } else {
            Color::from_rgb_hex(0x303236)
        };
        let primary_pressed = if dark {
            Color::from_rgb_hex(0xd9dce1)
        } else {
            Color::BLACK
        };
        let primary_foreground = if dark { Color::BLACK } else { Color::WHITE };
        let ghost_hover = colors.text_primary.with_alpha(if dark { 24 } else { 14 });
        let ghost_pressed = colors.text_primary.with_alpha(if dark { 40 } else { 28 });

        Self {
            standard: ButtonPalette {
                rest: ControlAppearance {
                    background: colors.surface,
                    border: colors.border,
                    foreground: colors.text_primary,
                },
                hovered: ControlAppearance {
                    background: colors.surface_subtle,
                    border: colors.border_strong,
                    foreground: colors.text_primary,
                },
                pressed: ControlAppearance {
                    background: colors.surface_muted,
                    border: colors.border_strong,
                    foreground: colors.text_primary,
                },
            },
            primary: solid_palette(
                primary_rest,
                primary_hover,
                primary_pressed,
                primary_foreground,
            ),
            accent: solid_palette(
                colors.accent,
                colors.accent_hovered,
                colors.accent_pressed,
                Color::WHITE,
            ),
            ghost: ButtonPalette {
                rest: ControlAppearance {
                    background: transparent,
                    border: transparent,
                    foreground: colors.text_primary,
                },
                hovered: ControlAppearance {
                    background: ghost_hover,
                    border: transparent,
                    foreground: colors.text_primary,
                },
                pressed: ControlAppearance {
                    background: ghost_pressed,
                    border: transparent,
                    foreground: colors.text_primary,
                },
            },
            danger: solid_palette(
                colors.destructive,
                colors.destructive_hovered,
                colors.destructive,
                Color::WHITE,
            ),
            horizontal_padding: 12.0,
            height: 32.0,
            radius: CornerRadius::Medium,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
            focus_ring: colors.focus_ring,
            focus_ring_width: 3.0,
            disabled_opacity: 0.42,
        }
    }
}

/// Visual and dimensional recipe shared by every standard text field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFieldTokens {
    pub background: Color,
    pub disabled_background: Color,
    pub border: Color,
    pub hovered_border: Color,
    pub focused_border: Color,
    pub invalid_border: Color,
    pub foreground: Color,
    pub placeholder_foreground: Color,
    pub disabled_foreground: Color,
    pub focus_ring: Color,
    pub selection: Color,
    pub caret: Color,
    pub horizontal_padding: f32,
    pub min_width: f32,
    pub radius: CornerRadius,
    pub stroke_width: f32,
    pub focus_ring_width: f32,
    pub selection_radius: f32,
    pub caret_width: f32,
}

impl TextFieldTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.surface,
            disabled_background: colors.surface_subtle,
            border: colors.border_strong,
            hovered_border: colors.text_secondary,
            focused_border: colors.accent,
            invalid_border: colors.destructive,
            foreground: colors.text_primary,
            placeholder_foreground: colors.text_tertiary,
            disabled_foreground: colors.text_disabled,
            focus_ring: colors.focus_ring,
            selection: colors.accent_soft,
            caret: colors.accent,
            horizontal_padding: FigmaTokens::SPACING.space_12,
            min_width: 100.0,
            radius: CornerRadius::Small,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
            focus_ring_width: 3.0,
            selection_radius: 3.0,
            caret_width: 2.0,
        }
    }
}

/// Standard list-row recipe, including selection and secondary content colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListTokens {
    pub background: Color,
    pub hovered_background: Color,
    pub selected_background: Color,
    pub foreground: Color,
    pub selected_foreground: Color,
    pub secondary_foreground: Color,
    pub leading_foreground: Color,
    pub status_marker: Color,
    pub row_padding: f32,
}

impl ListTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered_background: colors.surface_subtle,
            selected_background: colors.accent_soft,
            foreground: colors.text_primary,
            selected_foreground: colors.accent,
            secondary_foreground: colors.text_secondary,
            leading_foreground: colors.text_secondary,
            status_marker: colors.accent,
            row_padding: FigmaTokens::SPACING.space_8,
        }
    }
}

/// Standard modal surface recipe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialogTokens {
    pub background: Color,
    pub border: Color,
    pub padding: f32,
    pub radius: CornerRadius,
    pub stroke_width: f32,
}

impl DialogTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.elevated_surface,
            border: colors.border_strong,
            padding: FigmaTokens::SPACING.space_16,
            radius: CornerRadius::Large,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

/// Shared recipe for checkbox and radio interaction surfaces and indicators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionControlTokens {
    pub interaction_background: Color,
    pub interaction_hovered_background: Color,
    pub foreground: Color,
    pub disabled_foreground: Color,
    pub selected: Color,
    pub disabled_selected: Color,
    pub indicator_background: Color,
    pub disabled_indicator_background: Color,
    pub border: Color,
    pub disabled_border: Color,
    pub content_padding: f32,
    pub interaction_radius: CornerRadius,
    pub stroke_width: f32,
}

impl SelectionControlTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            interaction_background: Color::TRANSPARENT,
            interaction_hovered_background: colors.text_primary.with_alpha(14),
            foreground: colors.text_primary,
            disabled_foreground: colors.text_disabled,
            selected: colors.accent,
            disabled_selected: colors.accent.with_alpha(107),
            indicator_background: colors.surface,
            disabled_indicator_background: colors.surface_subtle,
            border: colors.border_strong,
            disabled_border: colors.border,
            content_padding: FigmaTokens::SPACING.space_4,
            interaction_radius: CornerRadius::Small,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchTokens {
    pub interaction_background: Color,
    pub foreground: Color,
    pub disabled_foreground: Color,
    pub off: Color,
    pub off_hovered: Color,
    pub off_pressed: Color,
    pub on: Color,
    pub on_hovered: Color,
    pub on_pressed: Color,
    pub knob: Color,
    pub disabled_knob: Color,
    pub knob_shadow: ShadowStyle,
    pub disabled_opacity: f32,
    pub content_padding: f32,
}

impl SwitchTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            interaction_background: Color::TRANSPARENT,
            foreground: colors.text_primary,
            disabled_foreground: colors.text_disabled,
            off: colors.surface_muted,
            off_hovered: colors.border,
            off_pressed: colors.border_strong,
            on: colors.accent,
            on_hovered: colors.accent_hovered,
            on_pressed: colors.accent_pressed,
            knob: Color::WHITE,
            disabled_knob: Color::rgba(255, 255, 255, 170),
            knob_shadow: ShadowStyle::Custom(ShadowSet::double(
                Shadow::new(Color::rgba(0, 0, 0, 28), 0.0, 1.0, 2.0, 0.0),
                Shadow::new(Color::rgba(0, 0, 0, 14), 0.0, 2.0, 4.0, 0.0),
            )),
            disabled_opacity: 0.45,
            content_padding: FigmaTokens::SPACING.space_4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderTokens {
    pub label: Color,
    pub disabled_label: Color,
    pub track: Color,
    pub fill: Color,
    pub hovered_fill: Color,
    pub pressed_fill: Color,
    pub knob: Color,
    pub hovered_knob: Color,
    pub disabled_knob: Color,
    pub knob_border: Color,
    pub hovered_knob_border: Color,
    pub focus_ring: Color,
    pub disabled_opacity: f32,
    pub stroke_width: f32,
    pub focus_ring_width: f32,
}

impl SliderTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            label: colors.text_primary,
            disabled_label: colors.text_disabled,
            track: colors.surface_muted,
            fill: colors.accent,
            hovered_fill: colors.accent_hovered,
            pressed_fill: colors.accent_pressed,
            knob: colors.elevated_surface,
            hovered_knob: colors.surface,
            disabled_knob: colors.surface_subtle,
            knob_border: colors.border,
            hovered_knob_border: colors.accent,
            focus_ring: colors.focus_ring,
            disabled_opacity: 0.45,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
            focus_ring_width: 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentedControlTokens {
    pub background: Color,
    pub border: Color,
    pub indicator_background: Color,
    pub indicator_border: Color,
    pub radius: CornerRadius,
    pub stroke_width: f32,
}

impl SegmentedControlTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.surface_subtle,
            border: colors.border,
            indicator_background: colors.surface,
            indicator_border: colors.border,
            radius: CornerRadius::ExtraLarge,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopoverTokens {
    pub background: Color,
    pub border: Color,
    pub padding: f32,
    pub radius: CornerRadius,
    pub stroke_width: f32,
}

impl PopoverTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.elevated_surface,
            border: colors.border_strong,
            padding: FigmaTokens::SPACING.space_8,
            radius: CornerRadius::Medium,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerTokens {
    pub background: Color,
    pub hovered_background: Color,
    pub border: Color,
    pub hovered_border: Color,
    pub foreground: Color,
    pub disabled_foreground: Color,
    pub indicator: Color,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub radius: CornerRadius,
}

impl PickerTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.surface,
            hovered_background: colors.surface_subtle,
            border: colors.border,
            hovered_border: colors.border_strong,
            foreground: colors.text_primary,
            disabled_foreground: colors.text_disabled,
            indicator: colors.text_secondary,
            horizontal_padding: FigmaTokens::SPACING.space_8,
            vertical_padding: FigmaTokens::SPACING.space_2,
            radius: CornerRadius::Small,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuTokens {
    pub item_background: Color,
    pub item_hovered_background: Color,
    pub danger_hovered_background: Color,
    pub foreground: Color,
    pub secondary_foreground: Color,
    pub disabled_foreground: Color,
    pub danger_foreground: Color,
    pub item_horizontal_padding: f32,
    pub item_vertical_padding: f32,
    pub item_radius: CornerRadius,
    pub surface_radius: CornerRadius,
    pub surface_stroke_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabsTokens {
    pub background: Color,
    pub hovered_background: Color,
    pub selected_background: Color,
    pub foreground: Color,
    pub selected_foreground: Color,
    pub disabled_foreground: Color,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub radius: CornerRadius,
}

impl TabsTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered_background: colors.surface_subtle,
            selected_background: colors.accent_soft,
            foreground: colors.text_secondary,
            selected_foreground: colors.accent,
            disabled_foreground: colors.text_disabled,
            horizontal_padding: FigmaTokens::SPACING.space_8,
            vertical_padding: FigmaTokens::SPACING.space_2,
            radius: CornerRadius::Small,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepperTokens {
    pub background: Color,
    pub hovered_background: Color,
    pub border: Color,
    pub hovered_border: Color,
    pub foreground: Color,
    pub disabled_foreground: Color,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub radius: CornerRadius,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressBarTokens {
    pub track: Color,
    pub disabled_track: Color,
    pub fill: Color,
    pub disabled_fill: Color,
    pub radius: CornerRadius,
}

impl ProgressBarTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            track: colors.surface_subtle,
            disabled_track: colors.border,
            fill: colors.accent,
            disabled_fill: colors.text_tertiary,
            radius: CornerRadius::Full,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipTokens {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub minimum_height: f32,
    pub radius: CornerRadius,
    pub stroke_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MessageBubbleTokens {
    pub received_background: Color,
    pub received_foreground: Color,
    pub sent_background: Color,
    pub sent_foreground: Color,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub radius: CornerRadius,
}

/// Visual recipe for compact, non-interactive status badges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeTokens {
    pub neutral_background: Color,
    pub neutral_foreground: Color,
    pub accent_background: Color,
    pub accent_foreground: Color,
    pub success_background: Color,
    pub warning_background: Color,
    pub error_background: Color,
    pub semantic_foreground: Color,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub radius: CornerRadius,
}

impl BadgeTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            neutral_background: colors.surface_subtle,
            neutral_foreground: colors.text_secondary,
            accent_background: colors.accent_soft,
            accent_foreground: colors.accent,
            success_background: colors.success,
            warning_background: colors.warning,
            error_background: colors.destructive,
            semantic_foreground: Color::WHITE,
            horizontal_padding: FigmaTokens::SPACING.space_8,
            vertical_padding: FigmaTokens::SPACING.space_2,
            radius: CornerRadius::Small,
        }
    }
}

/// Default presentation for identity avatars. Explicit component colors can
/// still override this recipe when an application has a meaningful identity
/// color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvatarTokens {
    pub background: Color,
    pub foreground: Color,
}

/// Default chrome and spacing for standard cards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardTokens {
    pub background: Color,
    pub border: Color,
    pub padding: f32,
    pub compact_padding: f32,
    pub radius: CornerRadius,
    pub shadow: ShadowStyle,
    pub stroke_width: f32,
}

impl CardTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.surface,
            border: colors.border,
            padding: FigmaTokens::SPACING.space_16,
            compact_padding: FigmaTokens::SPACING.space_8,
            radius: CornerRadius::Card,
            shadow: ShadowStyle::Card,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

/// Semantic backgrounds and floating chrome used by standard surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceTokens {
    pub app_background: Color,
    pub pane_background: Color,
    pub sidebar_background: Color,
    pub floating_background: Color,
    pub floating_border: Color,
    pub floating_radius: CornerRadius,
    pub floating_shadow: ShadowStyle,
    pub floating_stroke_width: f32,
}

impl SurfaceTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            app_background: colors.background,
            pane_background: colors.surface,
            sidebar_background: colors.surface_subtle,
            floating_background: colors.elevated_surface,
            floating_border: colors.border,
            floating_radius: CornerRadius::Card,
            floating_shadow: ShadowStyle::Floating,
            floating_stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

impl AvatarTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.surface_subtle,
            foreground: colors.text_secondary,
        }
    }
}

impl MessageBubbleTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            received_background: colors.surface_subtle,
            received_foreground: colors.text_primary,
            sent_background: colors.accent,
            sent_foreground: Color::WHITE,
            horizontal_padding: FigmaTokens::SPACING.space_12,
            vertical_padding: FigmaTokens::SPACING.space_8,
            radius: CornerRadius::ExtraLarge,
        }
    }
}

impl TooltipTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.elevated_surface,
            foreground: colors.text_primary,
            border: colors.border,
            horizontal_padding: FigmaTokens::SPACING.space_8 + FigmaTokens::SHAPE.stroke_default,
            vertical_padding: FigmaTokens::SPACING.space_2,
            minimum_height: 24.0,
            radius: CornerRadius::Small,
            stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

impl StepperTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            background: colors.surface,
            hovered_background: colors.surface_subtle,
            border: colors.border,
            hovered_border: colors.border_strong,
            foreground: colors.text_primary,
            disabled_foreground: colors.text_disabled,
            horizontal_padding: FigmaTokens::SPACING.space_8,
            vertical_padding: FigmaTokens::SPACING.space_2,
            radius: CornerRadius::Small,
        }
    }
}

impl MenuTokens {
    pub const fn from_colors(colors: ColorTokens) -> Self {
        Self {
            item_background: Color::TRANSPARENT,
            item_hovered_background: colors.accent_soft,
            danger_hovered_background: colors.destructive_soft,
            foreground: colors.text_primary,
            secondary_foreground: colors.text_tertiary,
            disabled_foreground: colors.text_disabled,
            danger_foreground: colors.destructive,
            item_horizontal_padding: FigmaTokens::SPACING.space_8,
            item_vertical_padding: FigmaTokens::SPACING.space_2,
            item_radius: CornerRadius::Small,
            surface_radius: CornerRadius::Medium,
            surface_stroke_width: FigmaTokens::SHAPE.stroke_default,
        }
    }
}

const fn solid_palette(
    rest: Color,
    hovered: Color,
    pressed: Color,
    foreground: Color,
) -> ButtonPalette {
    ButtonPalette {
        rest: ControlAppearance {
            background: rest,
            border: rest,
            foreground,
        },
        hovered: ControlAppearance {
            background: hovered,
            border: hovered,
            foreground,
        },
        pressed: ControlAppearance {
            background: pressed,
            border: pressed,
            foreground,
        },
    }
}
