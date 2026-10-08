use super::FigmaTokens;
use crate::geometry::Size;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutTokens {
    pub navigation_sidebar_width: f32,
    pub top_bar_height: f32,
    pub page_margin: f32,
    pub content_max_width: f32,
    pub form_width: f32,
    pub section_gap: f32,
    pub standard_window_width: f32,
    pub standard_window_height: f32,
    pub compact_window_width: f32,
    pub compact_window_height: f32,
    pub status_bar_height: f32,
    pub form_control_width: f32,
    pub compact_form_control_width: f32,
    pub settings_row_height: f32,
    pub settings_description_row_height: f32,
    pub toolbar_navigation_width: f32,
    pub browser_grid_cell_width: f32,
    pub browser_grid_cell_height: f32,
    pub bottom_bar_height: f32,
    pub list_row_height: f32,
    pub list_leading_size: f32,
    pub status_marker_size: f32,
    pub avatar_small_size: f32,
    pub avatar_size: f32,
    pub icon_button_size: f32,
    pub prominent_icon_button_size: f32,
    pub checkbox_size: f32,
    pub checkbox_glyph_size: f32,
    pub checkbox_radius: f32,
    pub compact_control_height: f32,
    pub control_height: f32,
    pub large_control_height: f32,
    pub dialog_width: f32,
    pub dialog_height: f32,
    pub popover_width: f32,
    pub popover_height: f32,
    pub control_min_width: f32,
    pub tab_width: f32,
    pub compact_icon_size: f32,
    pub control_icon_size: f32,
    pub icon_optical_scale: f32,
    pub stepper_icon_size: f32,
    pub radio_size: f32,
    pub radio_inset: f32,
    pub radio_dot_size: f32,
    pub range_control_width: f32,
    pub range_control_height: f32,
    pub range_track_height: f32,
    pub message_max_width: f32,
    pub control_thumb_height: f32,
    pub control_thumb_aspect_ratio: f32,
    pub control_pressed_thumb_aspect_ratio: f32,
    pub range_hit_padding: f32,
    pub switch_track_width: f32,
    pub switch_track_height: f32,
    pub switch_knob_width: f32,
    pub switch_knob_height: f32,
    pub switch_pressed_knob_width: f32,
    pub switch_knob_inset: f32,
    pub switch_drag_threshold: f32,
    pub switch_hit_padding: f32,
    pub segmented_control_inset: f32,
    pub segmented_control_height: f32,
    pub segmented_item_min_width: f32,
}

impl LayoutTokens {
    pub const DEFAULT: Self = Self {
        navigation_sidebar_width: FigmaTokens::LAYOUT.sidebar_width,
        top_bar_height: FigmaTokens::LAYOUT.toolbar_height,
        page_margin: FigmaTokens::LAYOUT.page_margin,
        content_max_width: FigmaTokens::LAYOUT.content_max_width,
        form_width: FigmaTokens::LAYOUT.form_width,
        section_gap: FigmaTokens::LAYOUT.section_gap,
        standard_window_width: FigmaTokens::LAYOUT.sidebar_width
            + FigmaTokens::LAYOUT.content_max_width
            + FigmaTokens::LAYOUT.page_margin * 2.0,
        standard_window_height: FigmaTokens::LAYOUT.content_max_width
            + FigmaTokens::LAYOUT.toolbar_height,
        compact_window_width: FigmaTokens::LAYOUT.content_max_width
            + FigmaTokens::LAYOUT.page_margin * 2.0,
        compact_window_height: FigmaTokens::LAYOUT.form_width + FigmaTokens::LAYOUT.toolbar_height,
        status_bar_height: FigmaTokens::SPACING.space_32,
        form_control_width: FigmaTokens::LAYOUT.sidebar_width + FigmaTokens::SPACING.space_24,
        compact_form_control_width: FigmaTokens::LAYOUT.sidebar_width
            - FigmaTokens::SPACING.space_48,
        settings_row_height: FigmaTokens::LAYOUT.toolbar_height,
        settings_description_row_height: FigmaTokens::SPACING.space_64,
        toolbar_navigation_width: FigmaTokens::LAYOUT.toolbar_height * 3.0,
        browser_grid_cell_width: FigmaTokens::LAYOUT.sidebar_width / 2.0,
        browser_grid_cell_height: FigmaTokens::SPACING.space_64 + FigmaTokens::SPACING.space_48,
        bottom_bar_height: FigmaTokens::LAYOUT.toolbar_height,
        list_row_height: FigmaTokens::SPACING.space_64,
        list_leading_size: FigmaTokens::SPACING.space_24,
        status_marker_size: FigmaTokens::SPACING.space_8,
        avatar_small_size: FigmaTokens::SPACING.space_24,
        avatar_size: FigmaTokens::SPACING.space_32,
        icon_button_size: 28.0,
        prominent_icon_button_size: FigmaTokens::SPACING.space_32,
        checkbox_size: 14.0,
        checkbox_glyph_size: 10.0,
        checkbox_radius: 4.0,
        compact_control_height: 26.0,
        control_height: 30.0,
        large_control_height: 36.0,
        dialog_width: FigmaTokens::LAYOUT.form_width - FigmaTokens::SPACING.space_24 * 5.0,
        dialog_height: FigmaTokens::LAYOUT.sidebar_width - FigmaTokens::SPACING.space_40,
        popover_width: FigmaTokens::LAYOUT.sidebar_width,
        popover_height: FigmaTokens::LAYOUT.sidebar_width / 2.0,
        control_min_width: FigmaTokens::LAYOUT.sidebar_width / 2.0,
        tab_width: 100.0,
        compact_icon_size: FigmaTokens::SPACING.space_16,
        control_icon_size: FigmaTokens::SPACING.space_16 + FigmaTokens::SPACING.space_4,
        icon_optical_scale: 0.84,
        stepper_icon_size: FigmaTokens::SPACING.space_24,
        radio_size: 16.0,
        radio_inset: 3.0,
        radio_dot_size: 6.0,
        range_control_width: FigmaTokens::LAYOUT.page_margin * 4.0,
        range_control_height: 28.0,
        range_track_height: 6.0,
        message_max_width: FigmaTokens::LAYOUT.form_width - FigmaTokens::SPACING.space_24 * 5.0,
        control_thumb_height: 20.0,
        control_thumb_aspect_ratio: 1.3,
        control_pressed_thumb_aspect_ratio: 1.5,
        range_hit_padding: FigmaTokens::SPACING.space_8,
        switch_track_width: 40.0,
        switch_track_height: 22.0,
        switch_knob_width: 23.0,
        switch_knob_height: 18.0,
        switch_pressed_knob_width: 26.2,
        switch_knob_inset: FigmaTokens::SPACING.space_2,
        switch_drag_threshold: FigmaTokens::SPACING.space_2,
        switch_hit_padding: FigmaTokens::SPACING.space_8,
        segmented_control_inset: FigmaTokens::SPACING.space_2,
        segmented_control_height: 30.0,
        segmented_item_min_width: 64.0,
    };

    pub fn control_thumb_size(self, pressed: bool) -> Size {
        let height = if self.control_thumb_height.is_finite() && self.control_thumb_height > 0.0 {
            self.control_thumb_height
        } else {
            FigmaTokens::SHAPE.radius_xlarge
        };
        let configured_ratio = if pressed {
            self.control_pressed_thumb_aspect_ratio
        } else {
            self.control_thumb_aspect_ratio
        };
        let ratio = if configured_ratio.is_finite() && configured_ratio > 0.0 {
            configured_ratio
        } else {
            1.0
        };
        Size::new(height * ratio, height)
    }
}
