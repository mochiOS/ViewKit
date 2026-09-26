//! 単一行のテキストフィールド

use super::{BorderStyle, Icon, Rectangle, RectangleColor, SymbolName, Text};
use crate::command::{CommandStatus, standard as commands};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::clipboard::{
    set_text as set_system_clipboard_text, text as system_clipboard_text,
};
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::typography::{TextRole, Typography};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const SECURE_GLYPH: char = '\u{2022}';

#[derive(Clone, Debug, Default, PartialEq)]
struct TextFieldInteractionInner {
    hovered: bool,
    focused: bool,
    enabled: bool,
    value: String,
    cursor: usize,
    scroll_offset_x: f32,
    selection_anchor: Option<usize>,
    selecting: bool,

    value_initialized: bool,
    caret_blink_origin: Option<Instant>,
}

#[derive(Clone)]
pub struct TextFieldInteractionState {
    inner: Rc<RefCell<TextFieldInteractionInner>>,
}

impl Default for TextFieldInteractionState {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(TextFieldInteractionInner {
                enabled: true,

                ..TextFieldInteractionInner::default()
            })),
        }
    }
}

impl TextFieldInteractionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_hovered(&self) -> bool {
        self.inner.borrow().hovered
    }

    pub fn is_focused(&self) -> bool {
        self.inner.borrow().focused
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.borrow().enabled
    }

    pub fn set_focused(&self, focused: bool) -> bool {
        let mut inner = self.inner.borrow_mut();

        let focused = focused && inner.enabled;

        let changed = inner.focused != focused;

        inner.focused = focused;

        changed
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();

        inner.hovered = false;
        inner.focused = false;
    }

    fn set_enabled(&self, enabled: bool) -> bool {
        let mut inner = self.inner.borrow_mut();

        let changed = inner.enabled != enabled;

        inner.enabled = enabled;

        if !enabled {
            inner.hovered = false;
            inner.focused = false;
        }

        changed
    }

    pub fn value(&self) -> String {
        self.inner.borrow().value.clone()
    }

    pub fn set_value(&self, value: impl Into<String>) {
        let value = value.into();
        let mut inner = self.inner.borrow_mut();

        inner.cursor = value.len();
        inner.scroll_offset_x = 0.0;
        inner.value = value;
        inner.value_initialized = true;
        inner.selection_anchor = None;
        inner.selecting = false;
    }

    /// 値が保持していた領域をNULで上書きしてから空にします。
    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        let byte_len = inner.value.len();
        if byte_len != 0 {
            inner.value.replace_range(.., &"\0".repeat(byte_len));
            inner.value.clear();
        }
        inner.cursor = 0;
        inner.scroll_offset_x = 0.0;
        inner.selection_anchor = None;
        inner.selecting = false;
    }

    fn initialize_value(&self, value: String) {
        let mut inner = self.inner.borrow_mut();

        if inner.value_initialized {
            return;
        }

        inner.cursor = value.len();
        inner.scroll_offset_x = 0.0;
        inner.value = value;
        inner.value_initialized = true;
        inner.selection_anchor = None;
        inner.selecting = false;
    }

    fn delete_forward(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        if delete_selection(&mut inner) {
            return true;
        }

        let cursor = inner.cursor.min(inner.value.len());

        if cursor >= inner.value.len() {
            return false;
        }

        let character_length = inner.value[cursor..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);

        if character_length == 0 {
            return false;
        }

        inner
            .value
            .replace_range(cursor..cursor + character_length, "");

        true
    }

    fn move_cursor_home(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        let changed = inner.cursor != 0 || inner.selection_anchor.is_some();

        inner.cursor = 0;
        inner.selection_anchor = None;
        inner.selecting = false;

        changed
    }

    fn move_cursor_end(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        let end = inner.value.len();
        let changed = inner.cursor != end || inner.selection_anchor.is_some();

        inner.cursor = end;
        inner.selection_anchor = None;
        inner.selecting = false;

        changed
    }

    fn insert_text(&self, text: &str) -> bool {
        let filtered: String = text
            .chars()
            .filter(|character| !character.is_control())
            .collect();

        if filtered.is_empty() {
            return false;
        }

        let mut inner = self.inner.borrow_mut();

        delete_selection(&mut inner);
        let cursor = inner.cursor.min(inner.value.len());
        inner.value.insert_str(cursor, filtered.as_str());
        inner.cursor = cursor + filtered.len();
        inner.selection_anchor = None;
        inner.selecting = false;

        true
    }

    fn extend_selection_left(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let cursor = inner.cursor.min(inner.value.len());

        if cursor == 0 {
            return false;
        }

        if inner.selection_anchor.is_none() {
            inner.selection_anchor = Some(cursor);
        }

        let previous = inner.value[..cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0);

        inner.cursor = previous;
        inner.selecting = false;

        true
    }

    fn extend_selection_right(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let cursor = inner.cursor.min(inner.value.len());

        if cursor >= inner.value.len() {
            return false;
        }

        if inner.selection_anchor.is_none() {
            inner.selection_anchor = Some(cursor);
        }

        let character_length = inner.value[cursor..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);

        if character_length == 0 {
            return false;
        }

        inner.cursor = cursor + character_length;

        inner.selecting = false;

        true
    }

    fn extend_selection_home(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let cursor = inner.cursor.min(inner.value.len());

        if cursor == 0 {
            return false;
        }

        if inner.selection_anchor.is_none() {
            inner.selection_anchor = Some(cursor);
        }

        inner.cursor = 0;
        inner.selecting = false;

        true
    }

    fn extend_selection_end(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let cursor = inner.cursor.min(inner.value.len());

        let end = inner.value.len();

        if cursor == end {
            return false;
        }

        if inner.selection_anchor.is_none() {
            inner.selection_anchor = Some(cursor);
        }

        inner.cursor = end;
        inner.selecting = false;

        true
    }

    fn select_all(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let end = inner.value.len();

        if end == 0 {
            inner.selection_anchor = None;
            inner.selecting = false;

            return false;
        }

        let changed = inner.selection_anchor != Some(0) || inner.cursor != end;

        inner.selection_anchor = Some(0);

        inner.cursor = end;
        inner.selecting = false;

        changed
    }

    fn delete_backward(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        if delete_selection(&mut inner) {
            return true;
        }

        let cursor = inner.cursor.min(inner.value.len());

        if cursor == 0 {
            return false;
        }

        let previous = inner.value[..cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0);

        inner.value.replace_range(previous..cursor, "");
        inner.cursor = previous;
        true
    }

    fn move_cursor_left(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        if let Some(range) = selection_range(&inner) {
            inner.cursor = range.start;
            inner.selection_anchor = None;
            inner.selecting = false;

            return true;
        }
        let cursor = inner.cursor.min(inner.value.len());

        if cursor == 0 {
            return false;
        }

        let previous = inner.value[..cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0);

        inner.cursor = previous;

        true
    }

    fn move_cursor_right(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        if let Some(range) = selection_range(&inner) {
            inner.cursor = range.end;
            inner.selection_anchor = None;
            inner.selecting = false;

            return true;
        }
        let cursor = inner.cursor.min(inner.value.len());

        if cursor >= inner.value.len() {
            return false;
        }

        let character_length = inner.value[cursor..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0);

        if character_length == 0 {
            return false;
        }

        inner.cursor = cursor + character_length;

        true
    }

    fn reset_caret_blink(&self) {
        self.inner.borrow_mut().caret_blink_origin = Some(Instant::now());
    }

    fn caret_blink_state(&self, now: Instant) -> (bool, Instant) {
        let mut inner = self.inner.borrow_mut();
        let origin = *inner.caret_blink_origin.get_or_insert(now);
        let elapsed = now.saturating_duration_since(origin);
        let interval_millis = CARET_BLINK_INTERVAL.as_millis();
        let elapsed_millis = elapsed.as_millis();
        let phase = elapsed_millis / interval_millis;
        let visible = phase.is_multiple_of(2);
        let remaining_millis = interval_millis - elapsed_millis % interval_millis;
        let next_redraw = now + Duration::from_millis(remaining_millis as u64);

        (visible, next_redraw)
    }

    fn stop_caret_blink(&self) {
        self.inner.borrow_mut().caret_blink_origin = None;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldSize {
    Small,

    #[default]
    Medium,

    Large,
}

impl TextFieldSize {
    fn height(self, theme: &crate::theme::Theme) -> f32 {
        match self {
            Self::Small => theme.layout.compact_control_height,
            Self::Medium => theme.layout.control_height,
            Self::Large => theme.layout.large_control_height,
        }
    }

    fn horizontal_padding(self, theme: &crate::theme::Theme) -> f32 {
        theme.text_field.horizontal_padding
    }

    const fn text_role(self) -> TextRole {
        match self {
            Self::Small => TextRole::Label,
            Self::Medium | Self::Large => TextRole::Body,
        }
    }

    fn text_style(self, typography: &Typography) -> crate::typography::TextStyle {
        typography.style(self.text_role())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TextFieldAppearance {
    background: Color,
    border: Color,
    foreground: Color,
}

pub struct TextField {
    interaction: TextFieldInteractionState,
    binding: Option<Binding<String>>,

    placeholder: String,

    size: TextFieldSize,
    radius: CornerRadius,
    leading_symbol: Option<SymbolName>,

    enabled: bool,
    invalid: bool,
    secure: bool,
    on_submit: Option<RefCell<Box<dyn FnMut()>>>,
}

impl TextField {
    #[must_use]
    pub fn new(binding: Binding<String>) -> Self {
        let interaction = TextFieldInteractionState::new();

        interaction.initialize_value(binding.get());

        Self {
            interaction,
            binding: Some(binding),

            placeholder: String::new(),
            size: TextFieldSize::Medium,
            radius: CornerRadius::Small,
            leading_symbol: None,

            enabled: true,
            invalid: false,
            secure: false,
            on_submit: None,
        }
    }

    #[must_use]
    pub fn with_interaction(interaction: TextFieldInteractionState) -> Self {
        Self {
            interaction,
            binding: None,

            placeholder: String::new(),
            size: TextFieldSize::Medium,
            radius: CornerRadius::Small,
            leading_symbol: None,

            enabled: true,
            invalid: false,
            secure: false,
            on_submit: None,
        }
    }

    pub fn value(self, value: impl Into<String>) -> Self {
        self.interaction.initialize_value(value.into());

        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();

        self
    }

    pub fn size(mut self, size: TextFieldSize) -> Self {
        self.size = size;
        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = radius;
        self
    }

    /// Adds a leading glyph sourced from VK Symbols. The text, selection and
    /// caret are inset automatically; applications must not draw replacement
    /// SVG artwork over a text field.
    pub fn leading_symbol(mut self, symbol: SymbolName) -> Self {
        self.leading_symbol = Some(symbol);
        self
    }

    pub fn capsule(mut self) -> Self {
        self.radius = CornerRadius::Full;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// 入力値を伏字で表示します。保持される値自体は変更しません。
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Enterキーで入力が確定されたときに呼ぶ処理を設定します。
    pub fn on_submit(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_submit = Some(RefCell::new(Box::new(callback)));
        self
    }

    pub fn interaction(&self) -> &TextFieldInteractionState {
        &self.interaction
    }

    fn display_text(&self) -> String {
        let value = self.interaction.value();

        if value.is_empty() {
            self.placeholder.clone()
        } else {
            self.display_value(&value)
        }
    }

    fn display_value(&self, value: &str) -> String {
        if self.secure {
            core::iter::repeat_n(SECURE_GLYPH, value.chars().count()).collect()
        } else {
            value.to_owned()
        }
    }

    fn display_prefix(&self, value: &str, byte_index: usize) -> String {
        self.display_value(&value[..byte_index.min(value.len())])
    }

    fn appearance(&self, context: &PaintContext<'_>) -> TextFieldAppearance {
        let interaction = self.interaction.inner.borrow();

        let background = if !interaction.enabled {
            context.theme.text_field.disabled_background
        } else {
            context.theme.text_field.background
        };

        let border = if self.invalid {
            context.theme.text_field.invalid_border
        } else if interaction.focused {
            context.theme.text_field.focused_border
        } else if interaction.hovered {
            context.theme.text_field.hovered_border
        } else {
            context.theme.text_field.border
        };

        let foreground = if !interaction.enabled {
            context.theme.text_field.disabled_foreground
        } else if interaction.value.is_empty() {
            context.theme.text_field.placeholder_foreground
        } else {
            context.theme.text_field.foreground
        };

        TextFieldAppearance {
            background,
            border,
            foreground,
        }
    }

    fn synchronize_binding(&self) {
        let Some(binding) = self.binding.as_ref() else {
            return;
        };

        binding.set_without_notification(self.interaction.value());
    }

    fn cursor_at_x(&self, pointer_x: f32, bounds: Rect, context: &mut EventContext<'_>) -> usize {
        let (value, scroll_offset_x) = {
            let inner = self.interaction.inner.borrow();

            (inner.value.clone(), inner.scroll_offset_x)
        };

        if value.is_empty() {
            return 0;
        }

        let text_origin_x = bounds.origin.x
            + self.size.horizontal_padding(context.theme)
            + self.leading_inset(context.theme);

        let target_x = (pointer_x - text_origin_x + scroll_offset_x).max(0.0);

        let mut previous_index = 0;
        let mut previous_width = 0.0;

        for (index, character) in value.char_indices() {
            let next_index = index + character.len_utf8();

            let prefix = self.display_prefix(&value, next_index);
            let next_width = Text::styled(prefix, self.size.text_role())
                .measure_unbounded_with_typography(context.text_measurer, context.typography)
                .width;

            let midpoint = (previous_width + next_width) / 2.0;

            if target_x < midpoint {
                return previous_index;
            }

            previous_index = next_index;
            previous_width = next_width;
        }

        value.len()
    }
}

impl TextField {
    fn leading_inset(&self, theme: &crate::theme::Theme) -> f32 {
        if self.leading_symbol.is_some() {
            theme.layout.compact_icon_size + theme.spacing.small
        } else {
            0.0
        }
    }
}

impl View for TextField {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let display_text = self.display_text();
        let text = Text::styled(display_text, self.size.text_role());

        let measured_text =
            text.measure_unbounded_with_typography(context.text_measurer, context.typography);

        let width = (measured_text.width
            + self.size.horizontal_padding(context.theme) * 2.0
            + self.leading_inset(context.theme))
        .max(context.theme.text_field.min_width);

        constraints.constrain(Size::new(width, self.size.height(context.theme)))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        self.interaction.set_enabled(self.enabled);

        let appearance = self.appearance(context);

        let (value, cursor, focused, stored_scroll_offset_x, selection) = {
            let inner = self.interaction.inner.borrow();

            (
                inner.value.clone(),
                inner.cursor.min(inner.value.len()),
                inner.focused && inner.enabled,
                inner.scroll_offset_x,
                selection_range(&inner),
            )
        };

        let has_selection = selection.is_some();
        context.record_command_status(CommandStatus::new(
            commands::COPY,
            bounds,
            focused && has_selection && !self.secure,
        ));
        context.record_command_status(CommandStatus::new(
            commands::CUT,
            bounds,
            focused && has_selection && !self.secure,
        ));
        context.record_command_status(CommandStatus::new(commands::PASTE, bounds, focused));
        context.record_command_status(CommandStatus::new(
            commands::DELETE,
            bounds,
            focused && (has_selection || cursor < value.len()),
        ));
        context.record_command_status(CommandStatus::new(
            commands::SELECT_ALL,
            bounds,
            focused && !value.is_empty(),
        ));

        let mut accessibility = AccessibilityNode::new(AccessibilityRole::TextField, bounds);
        if !self.placeholder.is_empty() {
            accessibility.label = Some(self.placeholder.clone());
        }
        if !self.secure {
            accessibility.value = Some(value.clone());
        }
        accessibility.enabled = self.enabled;
        accessibility.focusable = true;
        accessibility.focused = focused;
        accessibility.invalid = self.invalid;
        context.record_accessibility(accessibility);

        if focused {
            let ring_width = context.theme.text_field.focus_ring_width;

            let radius =
                self.radius
                    .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);

            let ring_bounds = Rect::new(
                bounds.origin.x - ring_width,
                bounds.origin.y - ring_width,
                bounds.size.width + ring_width * 2.0,
                bounds.size.height + ring_width * 2.0,
            );

            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.text_field.focus_ring))
                .radius(CornerRadius::Custom(radius + ring_width))
                .shadow(ShadowStyle::None)
                .border(BorderStyle::None)
                .paint(ring_bounds, context);
        }

        Rectangle::new()
            .color(RectangleColor::Custom(appearance.background))
            .radius(self.radius)
            .shadow(ShadowStyle::None)
            .border(BorderStyle::custom(
                appearance.border,
                context.theme.text_field.stroke_width,
            ))
            .paint(bounds, context);

        let showing_placeholder = value.is_empty();

        let secure_display = self.display_value(&value);
        let display_text = if showing_placeholder {
            self.placeholder.as_str()
        } else {
            secure_display.as_str()
        };

        let horizontal_padding = self.size.horizontal_padding(context.theme);
        let leading_inset = self.leading_inset(context.theme);

        if let Some(symbol) = self.leading_symbol {
            let icon_size = context.theme.layout.compact_icon_size;
            Icon::new(symbol)
                .size(icon_size)
                .color(context.theme.colors.text_secondary)
                .paint(
                    Rect::new(
                        bounds.origin.x + horizontal_padding,
                        bounds.origin.y + (bounds.size.height - icon_size).max(0.0) / 2.0,
                        icon_size,
                        icon_size,
                    ),
                    context,
                );
        }

        let text_style = self.size.text_style(context.typography);
        let line_height = text_style.line_height;

        let text_bounds = Rect::new(
            bounds.origin.x + horizontal_padding + leading_inset,
            bounds.origin.y
                + (bounds.size.height - line_height).max(0.0) / 2.0
                + context.theme.text_field.text_offset_y,
            (bounds.size.width - horizontal_padding * 2.0 - leading_inset).max(0.0),
            line_height.min(bounds.size.height),
        );

        let text_width = if value.is_empty() {
            0.0
        } else {
            Text::styled(display_text, self.size.text_role())
                .measure_unbounded_with_typography(context.text_measurer, context.typography)
                .width
        };

        let prefix_width = if cursor == 0 {
            0.0
        } else {
            Text::styled(self.display_prefix(&value, cursor), self.size.text_role())
                .measure_unbounded_with_typography(context.text_measurer, context.typography)
                .width
        };

        let viewport_width = text_bounds.size.width;

        let maximum_scroll_offset_x = (text_width - viewport_width).max(0.0);

        let mut scroll_offset_x = stored_scroll_offset_x.clamp(0.0, maximum_scroll_offset_x);

        if showing_placeholder {
            scroll_offset_x = 0.0;
        } else if focused {
            if prefix_width < scroll_offset_x {
                scroll_offset_x = prefix_width;
            } else if prefix_width > scroll_offset_x + viewport_width {
                scroll_offset_x = prefix_width - viewport_width;
            }

            scroll_offset_x = scroll_offset_x.clamp(0.0, maximum_scroll_offset_x);
        }

        if scroll_offset_x != stored_scroll_offset_x {
            self.interaction.inner.borrow_mut().scroll_offset_x = scroll_offset_x;
        }

        if !display_text.is_empty() {
            if showing_placeholder {
                Text::styled(display_text, self.size.text_role())
                    .accessibility_hidden(true)
                    .color(appearance.foreground)
                    .paint(text_bounds, context);
            } else {
                let content_bounds = Rect::new(
                    text_bounds.origin.x - scroll_offset_x,
                    text_bounds.origin.y,
                    text_width.max(viewport_width),
                    text_bounds.size.height,
                );

                let selection_bounds = if let Some(range) = selection.as_ref() {
                    let start_width = if range.start == 0 {
                        0.0
                    } else {
                        Text::styled(
                            self.display_prefix(&value, range.start),
                            self.size.text_role(),
                        )
                        .measure_unbounded_with_typography(
                            context.text_measurer,
                            context.typography,
                        )
                        .width
                    };

                    let end_width = Text::styled(
                        self.display_prefix(&value, range.end),
                        self.size.text_role(),
                    )
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;

                    let viewport_left = text_bounds.origin.x;
                    let viewport_right = text_bounds.origin.x + text_bounds.size.width;
                    let selection_left =
                        (text_bounds.origin.x + start_width - scroll_offset_x).max(viewport_left);
                    let selection_right =
                        (text_bounds.origin.x + end_width - scroll_offset_x).min(viewport_right);

                    if selection_right > selection_left {
                        let selection_height = (text_style.size
                            + context.theme.spacing.extra_small)
                            .min(text_bounds.size.height);
                        let selection_y = text_bounds.origin.y
                            + (text_bounds.size.height - selection_height) / 2.0;

                        Some(Rect::new(
                            selection_left,
                            selection_y,
                            selection_right - selection_left,
                            selection_height,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(selection_bounds) = selection_bounds {
                    context
                        .display_list
                        .push(DrawCommand::PushClip { rect: text_bounds });

                    context.display_list.push(DrawCommand::FillRoundedRect {
                        rect: selection_bounds,
                        radius: context.theme.text_field.selection_radius,
                        color: context.theme.text_field.selection,
                    });

                    context.display_list.push(DrawCommand::PopClip);
                }

                Text::styled(display_text, self.size.text_role())
                    .accessibility_hidden(true)
                    .color(appearance.foreground)
                    .paint(content_bounds, context);
            }
        }

        if selection.is_some() {
            self.interaction.stop_caret_blink();

            return;
        }

        if !focused {
            self.interaction.stop_caret_blink();
            return;
        }

        let now = Instant::now();

        let (caret_visible, next_redraw) = self.interaction.caret_blink_state(now);

        context.request_redraw_in_at(bounds.expanded(16.0), next_redraw);

        if !caret_visible {
            return;
        }

        let caret_width = context.theme.text_field.caret_width;
        let caret_half_width = caret_width / 2.0;

        let caret_min_x = text_bounds.origin.x;
        let caret_max_x = (text_bounds.origin.x + viewport_width).max(caret_min_x);
        let caret_center_x =
            (text_bounds.origin.x + prefix_width - scroll_offset_x).clamp(caret_min_x, caret_max_x);

        let caret_x = caret_center_x - caret_half_width;

        let caret_height = (line_height - 2.0)
            .max(12.0)
            .min((bounds.size.height - 8.0).max(1.0));

        let caret_y = bounds.origin.y + (bounds.size.height - caret_height) / 2.0;

        context.display_list.push(DrawCommand::FillRoundedRect {
            rect: Rect::new(caret_x, caret_y, caret_width, caret_height),
            radius: caret_half_width,
            color: context.theme.text_field.caret,
        });
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let enabled_changed = self.interaction.set_enabled(self.enabled);

        if enabled_changed {
            context.request_redraw_in(bounds.expanded(16.0));
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let should_focus = target.is_some_and(|target| target == bounds);
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != should_focus;
                inner.focused = should_focus && inner.enabled;
                if !should_focus {
                    inner.selection_anchor = None;
                    inner.selecting = false;
                    inner.caret_blink_origin = None;
                } else {
                    inner.caret_blink_origin = Some(Instant::now());
                }
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }
                EventResult::Ignored
            }

            ViewEvent::PointerMoved { position } => {
                let hovered = bounds.contains(*position);
                let selecting = {
                    let mut inner = self.interaction.inner.borrow_mut();
                    let hover_changed = inner.hovered != hovered;
                    inner.hovered = hovered;

                    if hover_changed {
                        context.request_redraw_in(bounds.expanded(16.0));
                    }

                    inner.selecting && inner.focused
                };

                if selecting {
                    let cursor = self.cursor_at_x(position.x, bounds, context);
                    let mut inner = self.interaction.inner.borrow_mut();
                    if inner.cursor != cursor {
                        inner.cursor = cursor;
                        inner.caret_blink_origin = Some(Instant::now());

                        context.request_redraw_in(bounds.expanded(16.0));
                    }
                }

                EventResult::Ignored
            }

            ViewEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } => {
                let mut inner = self.interaction.inner.borrow_mut();

                if !inner.selecting {
                    return EventResult::Ignored;
                }

                inner.selecting = false;

                if inner.selection_anchor == Some(inner.cursor) {
                    inner.selection_anchor = None;
                }

                drop(inner);
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerFocusRequested { position } => {
                let should_focus = bounds.contains(*position);
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != should_focus
                    || (!should_focus && inner.selection_anchor.is_some());

                inner.focused = should_focus && inner.enabled;

                if !should_focus {
                    inner.selection_anchor = None;
                    inner.selecting = false;
                    inner.caret_blink_origin = None;
                }

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !bounds.contains(*position) {
                    if self.interaction.set_focused(false) {
                        context.request_redraw_in(bounds.expanded(16.0));
                    }
                    return EventResult::Ignored;
                }

                let cursor = self.cursor_at_x(position.x, bounds, context);
                let mut inner = self.interaction.inner.borrow_mut();

                inner.hovered = true;
                inner.focused = true;
                inner.cursor = cursor;
                inner.selection_anchor = Some(cursor);
                inner.selecting = true;
                inner.caret_blink_origin = Some(Instant::now());

                drop(inner);

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.hovered;

                inner.hovered = false;

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            ViewEvent::FocusChanged { focused: false } => {
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.hovered || inner.focused;

                inner.hovered = false;
                inner.focused = false;

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            ViewEvent::TextInput { text } => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if self.interaction.insert_text(text) {
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Consumed
            }

            ViewEvent::KeyPressed {
                key: Key::Enter, ..
            } => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }
                if let Some(callback) = self.on_submit.as_ref() {
                    (callback.borrow_mut())();
                }
                EventResult::Consumed
            }

            ViewEvent::Backspace => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if self.interaction.delete_backward() {
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Consumed
            }

            ViewEvent::Delete => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if self.interaction.delete_forward() {
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Consumed
            }

            ViewEvent::ArrowLeft => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_left();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::ArrowRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_right();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::Home => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_home();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::End => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_end();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectLeft => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_left();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_right();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectHome => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_home();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectEnd => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_end();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::COPY =>
            {
                if !self.secure {
                    let selected = {
                        let inner = self.interaction.inner.borrow();
                        selection_range(&inner).map(|range| inner.value[range].to_owned())
                    };
                    if let Some(selected) = selected {
                        let _ = set_system_clipboard_text(&selected);
                    }
                }
                EventResult::Consumed
            }

            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::CUT =>
            {
                if !self.secure {
                    let selected = {
                        let inner = self.interaction.inner.borrow();
                        selection_range(&inner).map(|range| inner.value[range].to_owned())
                    };
                    if let Some(selected) = selected
                        && set_system_clipboard_text(&selected)
                    {
                        let changed = delete_selection(&mut self.interaction.inner.borrow_mut());
                        if changed {
                            self.synchronize_binding();
                            self.interaction.reset_caret_blink();
                            context.request_redraw_in(bounds.expanded(16.0));
                        }
                    }
                }
                EventResult::Consumed
            }

            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::PASTE =>
            {
                if let Some(text) = system_clipboard_text()
                    && self.interaction.insert_text(&text)
                {
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.request_redraw_in(bounds.expanded(16.0));
                }
                EventResult::Consumed
            }

            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::DELETE =>
            {
                if self.interaction.delete_forward() {
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.request_redraw_in(bounds.expanded(16.0));
                }
                EventResult::Consumed
            }

            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::SELECT_ALL =>
            {
                self.interaction.select_all();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::SelectAll => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.select_all();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            _ => EventResult::Ignored,
        }
    }
}

#[allow(unused)]
fn caret_is_visible() -> bool {
    static BLINK_EPOCH: OnceLock<Instant> = OnceLock::new();
    let elapsed_millis = BLINK_EPOCH.get_or_init(Instant::now).elapsed().as_millis();
    let interval_millis = CARET_BLINK_INTERVAL.as_millis();

    (elapsed_millis / interval_millis).is_multiple_of(2)
}

fn selection_range(inner: &TextFieldInteractionInner) -> Option<Range<usize>> {
    let anchor = inner.selection_anchor?;

    let cursor = inner.cursor.min(inner.value.len());

    let anchor = anchor.min(inner.value.len());

    if anchor == cursor {
        return None;
    }

    Some(anchor.min(cursor)..anchor.max(cursor))
}

fn delete_selection(inner: &mut TextFieldInteractionInner) -> bool {
    let Some(range) = selection_range(inner) else {
        inner.selection_anchor = None;

        return false;
    };

    inner.value.replace_range(range.clone(), "");

    inner.cursor = range.start;
    inner.selection_anchor = None;
    inner.selecting = false;

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_field_masks_each_character() {
        let interaction = TextFieldInteractionState::new();
        interaction.set_value("secretあ");
        let field = TextField::with_interaction(interaction).secure(true);

        assert_eq!(field.display_text(), "•••••••");
    }

    #[test]
    fn clearing_interaction_removes_secret_and_cursor_state() {
        let interaction = TextFieldInteractionState::new();
        interaction.set_value("secret");
        interaction.clear();

        assert_eq!(interaction.value(), "");
        assert_eq!(interaction.inner.borrow().cursor, 0);
        assert_eq!(interaction.inner.borrow().selection_anchor, None);
    }
}
