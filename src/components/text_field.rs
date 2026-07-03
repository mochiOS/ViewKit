//! 単一行のテキストフィールド

use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::PointerButton;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;

use super::{BorderStyle, Rectangle, RectangleColor, Text};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TextFieldInteractionInner {
    hovered: bool,
    focused: bool,
    enabled: bool,
    value: String,
    cursor: usize,
    value_initialized: bool,
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
        inner.value = value;

        inner.value_initialized = true;
    }

    fn initialize_value(&self, value: String) {
        let mut inner = self.inner.borrow_mut();

        if inner.value_initialized {
            return;
        }

        inner.cursor = value.len();
        inner.value = value;
        inner.value_initialized = true;
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

        let cursor = inner.cursor.min(inner.value.len());
        inner.value.insert_str(cursor, filtered.as_str());
        inner.cursor = cursor + filtered.len();

        true
    }

    fn delete_backward(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
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

        let (value, cursor, focused) = {
            let inner = self.interaction.inner.borrow();

            (
                inner.value.clone(),
                inner.cursor.min(inner.value.len()),
                inner.focused && inner.enabled,
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

        if !display_text.is_empty() {
            Text::new(display_text)
                .font_size(self.size.font_size())
                .line_height(line_height)
                .color(appearance.foreground)
                .paint(text_bounds, context);
        }

        if !focused {
            return;
        }

        let prefix_width = if cursor == 0 {
            0.0
        } else {
            Text::new(&value[..cursor])
                .font_size(self.size.font_size())
                .line_height(line_height)
                .measure_unbounded(context.text_measurer)
                .width
        };

        let caret_width = 2.0;

        let minimum_caret_x = text_bounds.origin.x;

        let maximum_caret_x =
            (text_bounds.origin.x + text_bounds.size.width - caret_width).max(minimum_caret_x);

        let caret_x = (text_bounds.origin.x + prefix_width).clamp(minimum_caret_x, maximum_caret_x);

        let caret_height = (line_height - 2.0)
            .max(12.0)
            .min((bounds.size.height - 8.0).max(1.0));

        let caret_y = bounds.origin.y + (bounds.size.height - caret_height) / 2.0;

        context.display_list.push(DrawCommand::FillRoundedRect {
            rect: Rect::new(caret_x, caret_y, caret_width, caret_height),

            radius: caret_width / 2.0,

            color: context.theme.colors.accent,
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

                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.hovered != hovered;

                inner.hovered = hovered;

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

                let mut inner = self.interaction.inner.borrow_mut();

                let changed = !inner.focused || !inner.hovered;

                inner.hovered = true;
                inner.focused = true;

                drop(inner);

                if changed {
                    context.request_redraw();
                }

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let inside = bounds.contains(*position);

                if !inside {
                    let mut inner = self.interaction.inner.borrow_mut();

                    let changed = inner.focused;

                    inner.focused = false;

                    drop(inner);

                    if changed {
                        context.request_redraw();
                    }

                    return EventResult::Ignored;
                }

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

                if self.interaction.insert_text(text) {
                    context.request_redraw();
                }

                EventResult::Consumed
            }

            ViewEvent::Backspace => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if self.interaction.delete_backward() {
                    context.request_redraw();
                }

                EventResult::Consumed
            }

            ViewEvent::ArrowLeft => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if self.interaction.move_cursor_left() {
                    context.request_redraw();
                }

                EventResult::Consumed
            }

            ViewEvent::ArrowRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if self.interaction.move_cursor_right() {
                    context.request_redraw();
                }

                EventResult::Consumed
            }

            _ => EventResult::Ignored,
        }
    }
}
