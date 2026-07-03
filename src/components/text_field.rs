//! 単一行のテキストフィールド

use super::{BorderStyle, Rectangle, RectangleColor, Text};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::PointerButton;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

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
        let visible = phase % 2 == 0;
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
    pub const fn height(self) -> f32 {
        match self {
            Self::Small => 28.0,
            Self::Medium => 36.0,
            Self::Large => 44.0,
        }
    }

    const fn horizontal_padding(self) -> f32 {
        match self {
            Self::Small => 9.0,
            Self::Medium => 11.0,
            Self::Large => 13.0,
        }
    }

    const fn font_size(self) -> f32 {
        match self {
            Self::Small => 12.0,
            Self::Medium => 13.0,
            Self::Large => 14.0,
        }
    }

    const fn line_height(self) -> f32 {
        match self {
            Self::Small => 18.0,
            Self::Medium => 20.0,
            Self::Large => 22.0,
        }
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

    placeholder: String,

    size: TextFieldSize,
    radius: CornerRadius,

    enabled: bool,
    invalid: bool,
}

impl TextField {
    pub fn new(interaction: TextFieldInteractionState) -> Self {
        Self {
            interaction,

            placeholder: String::new(),
            size: TextFieldSize::Medium,
            radius: CornerRadius::Medium,

            enabled: true,
            invalid: false,
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

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
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
            value
        }
    }

    fn appearance(&self, context: &PaintContext<'_>) -> TextFieldAppearance {
        let interaction = self.interaction.inner.borrow();

        let background = if !interaction.enabled {
            context.theme.colors.surface_subtle
        } else {
            context.theme.colors.surface
        };

        let border = if self.invalid {
            context.theme.colors.destructive
        } else if interaction.focused {
            context.theme.colors.accent
        } else if interaction.hovered {
            Color::rgba(0, 0, 0, 61)
        } else {
            context.theme.colors.border_strong
        };

        let foreground = if !interaction.enabled || interaction.value.is_empty() {
            context.theme.colors.text_tertiary
        } else {
            context.theme.colors.text_primary
        };

        TextFieldAppearance {
            background,
            border,
            foreground,
        }
    }

    fn cursor_at_x(&self, pointer_x: f32, bounds: Rect, context: &mut EventContext<'_>) -> usize {
        let (value, scroll_offset_x) = {
            let inner = self.interaction.inner.borrow();

            (inner.value.clone(), inner.scroll_offset_x)
        };

        if value.is_empty() {
            return 0;
        }

        let text_origin_x = bounds.origin.x + self.size.horizontal_padding();

        let target_x = (pointer_x - text_origin_x + scroll_offset_x).max(0.0);

        let mut previous_index = 0;
        let mut previous_width = 0.0;

        for (index, character) in value.char_indices() {
            let next_index = index + character.len_utf8();

            let next_width = Text::new(&value[..next_index])
                .font_size(self.size.font_size())
                .line_height(self.size.line_height())
                .measure_unbounded(context.text_measurer)
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

impl View for TextField {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let display_text = self.display_text();
        let text = Text::new(display_text)
            .font_size(self.size.font_size())
            .line_height(self.size.line_height());

        let measured_text = text.measure_unbounded(context.text_measurer);

        let width = (measured_text.width + self.size.horizontal_padding() * 2.0).max(160.0);

        constraints.constrain(Size::new(width, self.size.height()))
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

        if focused {
            let ring_width = 3.0;

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
                .color(RectangleColor::Custom(context.theme.colors.accent_soft))
                .radius(CornerRadius::Custom(radius + ring_width))
                .shadow(ShadowStyle::None)
                .border(BorderStyle::None)
                .paint(ring_bounds, context);
        }

        Rectangle::new()
            .color(RectangleColor::Custom(appearance.background))
            .radius(self.radius)
            .shadow(ShadowStyle::None)
            .border(BorderStyle::custom(appearance.border, 1.0))
            .paint(bounds, context);

        let showing_placeholder = value.is_empty();

        let display_text = if showing_placeholder {
            self.placeholder.as_str()
        } else {
            value.as_str()
        };

        let horizontal_padding = self.size.horizontal_padding();

        let line_height = self.size.line_height();

        let text_bounds = Rect::new(
            bounds.origin.x + horizontal_padding,
            bounds.origin.y + (bounds.size.height - line_height).max(0.0) / 2.0,
            (bounds.size.width - horizontal_padding * 2.0).max(0.0),
            line_height.min(bounds.size.height),
        );

        let text_width = if value.is_empty() {
            0.0
        } else {
            Text::new(value.as_str())
                .font_size(self.size.font_size())
                .line_height(line_height)
                .measure_unbounded(context.text_measurer)
                .width
        };

        let prefix_width = if cursor == 0 {
            0.0
        } else {
            Text::new(&value[..cursor])
                .font_size(self.size.font_size())
                .line_height(line_height)
                .measure_unbounded(context.text_measurer)
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
                Text::new(display_text)
                    .font_size(self.size.font_size())
                    .line_height(line_height)
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
                        Text::new(&value[..range.start])
                            .font_size(self.size.font_size())
                            .line_height(line_height)
                            .measure_unbounded(context.text_measurer)
                            .width
                    };

                    let end_width = Text::new(&value[..range.end])
                        .font_size(self.size.font_size())
                        .line_height(line_height)
                        .measure_unbounded(context.text_measurer)
                        .width;

                    let viewport_left = text_bounds.origin.x;
                    let viewport_right = text_bounds.origin.x + text_bounds.size.width;
                    let selection_left =
                        (text_bounds.origin.x + start_width - scroll_offset_x).max(viewport_left);
                    let selection_right =
                        (text_bounds.origin.x + end_width - scroll_offset_x).min(viewport_right);

                    if selection_right > selection_left {
                        let selection_height =
                            (self.size.font_size() + 4.0).min(text_bounds.size.height);
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
                        radius: 3.0,
                        color: context.theme.colors.accent.alpha(0.2),
                    });

                    context.display_list.push(DrawCommand::PopClip);
                }

                Text::new(display_text)
                    .font_size(self.size.font_size())
                    .line_height(line_height)
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

        context.request_redraw_at(next_redraw);

        if !caret_visible {
            return;
        }

        let caret_width = 2.0;
        let caret_half_width = caret_width / 2.0;

        let caret_center_x = (text_bounds.origin.x + prefix_width - scroll_offset_x)
            .clamp(text_bounds.origin.x, text_bounds.origin.x + viewport_width);

        let caret_x = caret_center_x - caret_half_width;

        let caret_height = (line_height - 2.0)
            .max(12.0)
            .min((bounds.size.height - 8.0).max(1.0));

        let caret_y = bounds.origin.y + (bounds.size.height - caret_height) / 2.0;

        context.display_list.push(DrawCommand::FillRoundedRect {
            rect: Rect::new(caret_x, caret_y, caret_width, caret_height),
            radius: caret_half_width,
            color: context.theme.colors.accent.alpha(0.5),
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
            context.request_redraw();
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            ViewEvent::PointerMoved { position } => {
                let hovered = bounds.contains(*position);
                let selecting = {
                    let mut inner = self.interaction.inner.borrow_mut();
                    let hover_changed = inner.hovered != hovered;
                    inner.hovered = hovered;

                    if hover_changed {
                        context.request_redraw();
                    }

                    inner.selecting && inner.focused
                };

                if selecting {
                    let cursor = self.cursor_at_x(position.x, bounds, context);
                    let mut inner = self.interaction.inner.borrow_mut();
                    if inner.cursor != cursor {
                        inner.cursor = cursor;
                        inner.caret_blink_origin = Some(Instant::now());

                        context.request_redraw();
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
                context.request_redraw();

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
                    context.request_redraw();
                }

                EventResult::Ignored
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !bounds.contains(*position) {
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

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.hovered;

                inner.hovered = false;

                drop(inner);

                if changed {
                    context.request_redraw();
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
                    context.request_redraw();
                }

                EventResult::Ignored
            }

            ViewEvent::TextInput { text } => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.insert_text(text);
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::Backspace => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.delete_backward();
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::Delete => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.delete_forward();
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::ArrowLeft => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_left();
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::ArrowRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_right();
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::Home => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_home();
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::End => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.move_cursor_end();
                self.interaction.reset_caret_blink();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::SelectLeft => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_left();

                self.interaction.reset_caret_blink();

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::SelectRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_right();

                self.interaction.reset_caret_blink();

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::SelectHome => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_home();

                self.interaction.reset_caret_blink();

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::SelectEnd => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.extend_selection_end();

                self.interaction.reset_caret_blink();

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::SelectAll => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction.select_all();

                self.interaction.reset_caret_blink();

                context.request_redraw();

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

    (elapsed_millis / interval_millis) % 2 == 0
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
