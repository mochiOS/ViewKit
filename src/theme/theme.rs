#![allow(clippy::missing_const_for_thread_local)]

//! ViewKit全体の外観テーマを定義

use super::{
    AvatarTokens, BadgeTokens, BrowserTokens, ButtonTokens, CardTokens, Color, ColorTokens,
    DialogTokens, DividerTokens, FigmaColorTokens, FigmaTokens, LayoutTokens, ListTokens, MenuTokens,
    MessageBubbleTokens, MotionTokens, PickerTokens, PopoverTokens, ProgressBarTokens, RadiusTokens,
    ScrollBarTokens, SegmentedControlTokens, SurfaceTokens,
    SelectionControlTokens, ShadowTokens, ShellTokens, SliderTokens, SpacingTokens,
    StepperTokens, SwitchTokens, TabsTokens, TextFieldTokens, TooltipTokens,
};
use crate::typography::Typography;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub typography: Typography,
    pub button: ButtonTokens,
    pub text_field: TextFieldTokens,
    pub list: ListTokens,
    pub dialog: DialogTokens,
    pub selection_control: SelectionControlTokens,
    pub switch: SwitchTokens,
    pub slider: SliderTokens,
    pub segmented_control: SegmentedControlTokens,
    pub popover: PopoverTokens,
    pub picker: PickerTokens,
    pub menu: MenuTokens,
    pub tabs: TabsTokens,
    pub stepper: StepperTokens,
    pub progress_bar: ProgressBarTokens,
    pub tooltip: TooltipTokens,
    pub message_bubble: MessageBubbleTokens,
    pub badge: BadgeTokens,
    pub avatar: AvatarTokens,
    pub card: CardTokens,
    pub surface: SurfaceTokens,
    pub radius: RadiusTokens,
    pub spacing: SpacingTokens,
    pub shadows: ShadowTokens,
    pub divider: DividerTokens,
    pub layout: LayoutTokens,
    pub scrollbar: ScrollBarTokens,
    pub motion: MotionTokens,
    pub shell: ShellTokens,
    pub browser: BrowserTokens,
}

impl Theme {
    pub const LIGHT: Self = Self {
        colors: colors_from_figma(FigmaTokens::LIGHT, false),
        typography: Typography::DEFAULT,
        button: ButtonTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false), false),
        text_field: TextFieldTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        list: ListTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        dialog: DialogTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        selection_control: SelectionControlTokens::from_colors(colors_from_figma(
            FigmaTokens::LIGHT,
            false,
        )),
        switch: SwitchTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        slider: SliderTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        segmented_control: SegmentedControlTokens::from_colors(colors_from_figma(
            FigmaTokens::LIGHT,
            false,
        )),
        popover: PopoverTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        picker: PickerTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        menu: MenuTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        tabs: TabsTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        stepper: StepperTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        progress_bar: ProgressBarTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        tooltip: TooltipTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        message_bubble: MessageBubbleTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        badge: BadgeTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        avatar: AvatarTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        card: CardTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),
        surface: SurfaceTokens::from_colors(colors_from_figma(FigmaTokens::LIGHT, false)),

        radius: RadiusTokens::DEFAULT,
        spacing: SpacingTokens::DEFAULT,
        shadows: ShadowTokens::DEFAULT,
        divider: DividerTokens::DEFAULT,
        layout: LayoutTokens::DEFAULT,
        scrollbar: ScrollBarTokens::DEFAULT,
        motion: MotionTokens::DEFAULT,
        shell: ShellTokens::LIGHT,
        browser: BrowserTokens::LIGHT,
    };
    pub const DARK: Self = Self {
        colors: colors_from_figma(FigmaTokens::DARK, true),
        typography: Typography::DEFAULT,
        button: ButtonTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true), true),
        text_field: TextFieldTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        list: ListTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        dialog: DialogTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        selection_control: SelectionControlTokens::from_colors(colors_from_figma(
            FigmaTokens::DARK,
            true,
        )),
        switch: SwitchTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        slider: SliderTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        segmented_control: SegmentedControlTokens::from_colors(colors_from_figma(
            FigmaTokens::DARK,
            true,
        )),
        popover: PopoverTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        picker: PickerTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        menu: MenuTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        tabs: TabsTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        stepper: StepperTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        progress_bar: ProgressBarTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        tooltip: TooltipTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        message_bubble: MessageBubbleTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        badge: BadgeTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        avatar: AvatarTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        card: CardTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),
        surface: SurfaceTokens::from_colors(colors_from_figma(FigmaTokens::DARK, true)),

        radius: RadiusTokens::DEFAULT,
        spacing: SpacingTokens::DEFAULT,
        shadows: ShadowTokens::DEFAULT,
        divider: DividerTokens::DEFAULT,
        layout: LayoutTokens::DEFAULT,
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
        self.button = ButtonTokens::from_colors(self.colors, dark);
        self.text_field = TextFieldTokens::from_colors(self.colors);
        self.list = ListTokens::from_colors(self.colors);
        self.dialog = DialogTokens::from_colors(self.colors);
        self.selection_control = SelectionControlTokens::from_colors(self.colors);
        self.switch = SwitchTokens::from_colors(self.colors);
        self.slider = SliderTokens::from_colors(self.colors);
        self.segmented_control = SegmentedControlTokens::from_colors(self.colors);
        self.popover = PopoverTokens::from_colors(self.colors);
        self.picker = PickerTokens::from_colors(self.colors);
        self.menu = MenuTokens::from_colors(self.colors);
        self.tabs = TabsTokens::from_colors(self.colors);
        self.stepper = StepperTokens::from_colors(self.colors);
        self.progress_bar = ProgressBarTokens::from_colors(self.colors);
        self.tooltip = TooltipTokens::from_colors(self.colors);
        self.message_bubble = MessageBubbleTokens::from_colors(self.colors);
        self.badge = BadgeTokens::from_colors(self.colors);
        self.avatar = AvatarTokens::from_colors(self.colors);
        self.card = CardTokens::from_colors(self.colors);
        self.surface = SurfaceTokens::from_colors(self.colors);
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

const fn colors_from_figma(tokens: FigmaColorTokens, dark: bool) -> ColorTokens {
    let soft_alpha = if dark { 48 } else { 24 };
    let focus_alpha = if dark { 102 } else { 72 };
    ColorTokens {
        background: tokens.background.primary,
        surface: tokens.background.primary,
        surface_subtle: tokens.background.secondary,
        surface_muted: tokens.border.subtle,
        elevated_surface: tokens.background.elevated,
        text_primary: tokens.text.primary,
        text_secondary: tokens.text.secondary,
        text_tertiary: tokens.text.tertiary,
        text_disabled: tokens.text.disabled,
        accent: tokens.accent.primary,
        accent_hovered: tokens.accent.hover,
        accent_pressed: tokens.accent.pressed,
        accent_soft: tokens.accent.subtle,
        border: tokens.border.default,
        border_strong: tokens.border.strong,
        focus_ring: tokens.accent.primary.with_alpha(focus_alpha),
        success: tokens.semantic.success,
        success_soft: tokens.semantic.success.with_alpha(soft_alpha),
        warning: tokens.semantic.warning,
        warning_soft: tokens.semantic.warning.with_alpha(soft_alpha),
        destructive: tokens.semantic.error.base,
        destructive_hovered: tokens.semantic.error.hover,
        destructive_soft: tokens.semantic.error.base.with_alpha(soft_alpha),
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
