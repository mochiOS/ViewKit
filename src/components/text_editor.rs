//! Multi-line editable text surface.

use super::{BorderStyle, Rectangle, RectangleColor, Text};
use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::platform::{CursorIcon, Key, PointerButton};
use crate::state::Binding;
use crate::theme::{CornerRadius, ShadowStyle};
use crate::typography::TextRole;
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const MAX_HISTORY: usize = 100;

#[derive(Clone, Debug, PartialEq)]
struct EditorSnapshot {
    value: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

#[derive(Debug)]
struct TextEditorInteractionInner {
    value: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    selecting: bool,
    focused: bool,
    hovered: bool,
    enabled: bool,
    initialized: bool,
    scroll_x: f32,
    scroll_y: f32,
    caret_blink_origin: Option<Instant>,
    undo: Vec<EditorSnapshot>,
    redo: Vec<EditorSnapshot>,
}

impl Default for TextEditorInteractionInner {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            selection_anchor: None,
            selecting: false,
            focused: false,
            hovered: false,
            enabled: true,
            initialized: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            caret_blink_origin: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct TextEditorInteractionState {
    inner: Rc<RefCell<TextEditorInteractionInner>>,
}

impl TextEditorInteractionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(&self) -> String {
        self.inner.borrow().value.clone()
    }

    pub fn set_value(&self, value: impl Into<String>) {
        let value = normalize_newlines(value.into());
        let mut inner = self.inner.borrow_mut();
        inner.value = value;
        inner.cursor = inner.value.len();
        inner.selection_anchor = None;
        inner.selecting = false;
        inner.scroll_x = 0.0;
        inner.scroll_y = 0.0;
        inner.undo.clear();
        inner.redo.clear();
        inner.initialized = true;
    }

    pub fn is_focused(&self) -> bool {
        self.inner.borrow().focused
    }

    pub fn can_undo(&self) -> bool {
        !self.inner.borrow().undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.inner.borrow().redo.is_empty()
    }

    pub fn perform_undo(&self) -> bool {
        self.undo()
    }

    pub fn perform_redo(&self) -> bool {
        self.redo()
    }

    fn initialize(&self, value: String) {
        if self.inner.borrow().initialized {
            return;
        }
        self.set_value(value);
    }

    fn snapshot(inner: &TextEditorInteractionInner) -> EditorSnapshot {
        EditorSnapshot {
            value: inner.value.clone(),
            cursor: inner.cursor,
            selection_anchor: inner.selection_anchor,
        }
    }

    fn restore(inner: &mut TextEditorInteractionInner, snapshot: EditorSnapshot) {
        inner.value = snapshot.value;
        inner.cursor = snapshot.cursor.min(inner.value.len());
        inner.selection_anchor = snapshot.selection_anchor;
        inner.selecting = false;
    }

    fn undo(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        let Some(snapshot) = inner.undo.pop() else {
            return false;
        };
        let current = Self::snapshot(&inner);
        inner.redo.push(current);
        Self::restore(&mut inner, snapshot);
        true
    }

    fn redo(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        let Some(snapshot) = inner.redo.pop() else {
            return false;
        };
        let current = Self::snapshot(&inner);
        inner.undo.push(current);
        Self::restore(&mut inner, snapshot);
        true
    }

    fn edit(&self, operation: impl FnOnce(&mut TextEditorInteractionInner) -> bool) -> bool {
        let mut inner = self.inner.borrow_mut();
        let before = Self::snapshot(&inner);
        if !operation(&mut inner) {
            return false;
        }
        if Self::snapshot(&inner) == before {
            return false;
        }
        if inner.value != before.value {
            if inner.undo.last() != Some(&before) {
                inner.undo.push(before);
                if inner.undo.len() > MAX_HISTORY {
                    inner.undo.remove(0);
                }
            }
            inner.redo.clear();
        }
        true
    }

    fn reset_caret(&self) {
        self.inner.borrow_mut().caret_blink_origin = Some(Instant::now());
    }
}

pub struct TextEditor {
    interaction: TextEditorInteractionState,
    binding: Option<Binding<String>>,
    placeholder: String,
    enabled: bool,
    monospaced: bool,
    bordered: bool,
    on_change: Option<RefCell<Box<dyn FnMut(&str)>>>,
}

impl TextEditor {
    pub fn new(binding: Binding<String>) -> Self {
        let interaction = TextEditorInteractionState::new();
        interaction.initialize(binding.get());
        Self {
            interaction,
            binding: Some(binding),
            placeholder: String::new(),
            enabled: true,
            monospaced: false,
            bordered: false,
            on_change: None,
        }
    }

    pub fn with_interaction(interaction: TextEditorInteractionState) -> Self {
        Self {
            interaction,
            binding: None,
            placeholder: String::new(),
            enabled: true,
            monospaced: false,
            bordered: false,
            on_change: None,
        }
    }

    pub fn value(self, value: impl Into<String>) -> Self {
        self.interaction.initialize(normalize_newlines(value.into()));
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn monospaced(mut self, monospaced: bool) -> Self {
        self.monospaced = monospaced;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn on_change(mut self, callback: impl FnMut(&str) + 'static) -> Self {
        self.on_change = Some(RefCell::new(Box::new(callback)));
        self
    }

    pub fn interaction(&self) -> &TextEditorInteractionState {
        &self.interaction
    }

    fn text(&self, value: impl Into<String>) -> Text {
        let text = Text::styled(value, TextRole::Body).cache_layout(false);
        if self.monospaced { text.monospaced() } else { text }
    }

    fn line_height(&self, context: &PaintContext<'_>) -> f32 {
        context.typography.style(TextRole::Body).line_height
            * context.text_measurer.font_scale()
    }

    fn event_line_height(&self, context: &EventContext<'_>) -> f32 {
        context.typography.style(TextRole::Body).line_height
            * context.text_measurer.font_scale()
    }

    fn content_bounds(bounds: Rect, padding: f32) -> Rect {
        Rect::new(
            bounds.origin.x + padding,
            bounds.origin.y + padding,
            (bounds.size.width - padding * 2.0).max(0.0),
            (bounds.size.height - padding * 2.0).max(0.0),
        )
    }

    fn synchronize(&self) {
        let value = self.interaction.value();
        if let Some(binding) = self.binding.as_ref() {
            binding.set_without_notification(value.clone());
        }
        if let Some(callback) = self.on_change.as_ref() {
            (callback.borrow_mut())(&value);
        }
    }

    fn index_at_point(&self, point: Point, bounds: Rect, context: &mut EventContext<'_>) -> usize {
        let inner = self.interaction.inner.borrow();
        let padding = context.theme.spacing.large;
        let line_height = self.event_line_height(context).max(1.0);
        let target_line = ((point.y - bounds.origin.y - padding + inner.scroll_y) / line_height)
            .floor()
            .max(0.0) as usize;
        let lines = line_ranges(&inner.value);
        let range = lines.get(target_line).or_else(|| lines.last()).cloned().unwrap_or(0..0);
        let line = &inner.value[range.clone()];
        let target_x = (point.x - bounds.origin.x - padding + inner.scroll_x).max(0.0);
        let mut previous = range.start;
        let mut previous_width = 0.0;
        for (relative, character) in line.char_indices() {
            let next = relative + character.len_utf8();
            let width = self.text(&line[..next])
                .measure_unbounded_with_typography(context.text_measurer, context.typography)
                .width;
            if target_x < (previous_width + width) / 2.0 {
                return previous;
            }
            previous = range.start + next;
            previous_width = width;
        }
        range.end
    }

    fn set_cursor(&self, cursor: usize, extend: bool) -> bool {
        let mut inner = self.interaction.inner.borrow_mut();
        let cursor = clamp_boundary(&inner.value, cursor);
        if extend && inner.selection_anchor.is_none() {
            inner.selection_anchor = Some(inner.cursor);
        } else if !extend {
            inner.selection_anchor = None;
        }
        let changed = inner.cursor != cursor;
        inner.cursor = cursor;
        inner.selecting = false;
        changed
    }

    fn move_horizontal(&self, right: bool, extend: bool) -> bool {
        let inner = self.interaction.inner.borrow();
        let cursor = inner.cursor;
        if !extend {
            if let Some(selection) = selection_range(&inner) {
                let target = if right { selection.end } else { selection.start };
                drop(inner);
                return self.set_cursor(target, false);
            }
        }
        let target = if right {
            next_boundary(&inner.value, cursor)
        } else {
            previous_boundary(&inner.value, cursor)
        };
        drop(inner);
        self.set_cursor(target, extend)
    }

    fn move_vertical(&self, down: bool, extend: bool) -> bool {
        let inner = self.interaction.inner.borrow();
        let lines = line_ranges(&inner.value);
        let (line_index, column) = line_and_column(&inner.value, inner.cursor, &lines);
        let target_line = if down {
            (line_index + 1).min(lines.len().saturating_sub(1))
        } else {
            line_index.saturating_sub(1)
        };
        let target = index_for_column(&inner.value, &lines[target_line], column);
        drop(inner);
        self.set_cursor(target, extend)
    }

    fn move_line_edge(&self, end: bool, extend: bool) -> bool {
        let inner = self.interaction.inner.borrow();
        let lines = line_ranges(&inner.value);
        let (line, _) = line_and_column(&inner.value, inner.cursor, &lines);
        let target = if end { lines[line].end } else { lines[line].start };
        drop(inner);
        self.set_cursor(target, extend)
    }

    fn select_all(&self) -> bool {
        let mut inner = self.interaction.inner.borrow_mut();
        if inner.value.is_empty() {
            return false;
        }
        let changed = inner.selection_anchor != Some(0) || inner.cursor != inner.value.len();
        inner.selection_anchor = Some(0);
        inner.cursor = inner.value.len();
        inner.selecting = false;
        changed
    }

    fn replace_selection(inner: &mut TextEditorInteractionInner, replacement: &str) -> bool {
        let range = selection_range(inner).unwrap_or(inner.cursor..inner.cursor);
        if range.is_empty() && replacement.is_empty() {
            return false;
        }
        inner.value.replace_range(range.clone(), replacement);
        inner.cursor = range.start + replacement.len();
        inner.selection_anchor = None;
        inner.selecting = false;
        true
    }
}

impl View for TextEditor {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(
            constraints.maximum.width.min(640.0),
            constraints.maximum.height.min(480.0),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }
        let padding = context.theme.spacing.large;
        let content = Self::content_bounds(bounds, padding);
        let mut inner = self.interaction.inner.borrow_mut();
        inner.enabled = self.enabled;
        let value = inner.value.clone();
        let cursor = inner.cursor.min(value.len());
        let selection = selection_range(&inner);
        let focused = inner.focused && inner.enabled;
        let line_height = self.line_height(context).max(1.0);
        let lines = line_ranges(&value);
        let content_height = lines.len() as f32 * line_height;
        let content_width = lines
            .iter()
            .map(|range| {
                self.text(&value[range.clone()])
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width
            })
            .fold(0.0_f32, f32::max);
        inner.scroll_x = inner.scroll_x.clamp(0.0, (content_width - content.size.width).max(0.0));
        inner.scroll_y = inner.scroll_y.clamp(0.0, (content_height - content.size.height).max(0.0));
        let scroll_x = inner.scroll_x;
        let scroll_y = inner.scroll_y;
        drop(inner);

        let mut node = AccessibilityNode::new(AccessibilityRole::TextField, bounds);
        node.label = Some(if self.placeholder.is_empty() { "Document".into() } else { self.placeholder.clone() });
        node.value = Some(value.clone());
        node.enabled = self.enabled;
        node.focusable = true;
        node.focused = focused;
        context.record_accessibility(node);

        Rectangle::new()
            .color(RectangleColor::Custom(context.theme.colors.surface))
            .radius(if self.bordered { CornerRadius::Small } else { CornerRadius::None })
            .shadow(ShadowStyle::None)
            .border(if self.bordered {
                BorderStyle::custom(context.theme.text_field.border, context.theme.text_field.stroke_width)
            } else {
                BorderStyle::None
            })
            .paint(bounds, context);

        context.display_list.push(DrawCommand::PushClip { rect: content });
        if let Some(selection) = selection.as_ref() {
            for (line_index, range) in lines.iter().enumerate() {
                let start = selection.start.max(range.start);
                let end = selection.end.min(range.end);
                if start >= end && !(selection.end > range.end && selection.start <= range.end) {
                    continue;
                }
                let line = &value[range.clone()];
                let start_width = self.text(&line[..start.saturating_sub(range.start)])
                    .measure_unbounded_with_typography(context.text_measurer, context.typography).width;
                let end_width = self.text(&line[..end.saturating_sub(range.start)])
                    .measure_unbounded_with_typography(context.text_measurer, context.typography).width;
                let width = (end_width - start_width).max(if start == end { 3.0 } else { 0.0 });
                context.display_list.push(DrawCommand::FillRoundedRect {
                    rect: Rect::new(
                        content.origin.x + start_width - scroll_x,
                        content.origin.y + line_index as f32 * line_height - scroll_y,
                        width,
                        line_height,
                    ),
                    radius: context.theme.text_field.selection_radius,
                    color: context.theme.text_field.selection,
                });
            }
        }

        if value.is_empty() && !focused && !self.placeholder.is_empty() {
            Text::body(self.placeholder.clone())
                .color(context.theme.text_field.placeholder_foreground)
                .accessibility_hidden(true)
                .paint(content, context);
        } else if !value.is_empty() {
            self.text(value.clone())
                .color(context.theme.colors.text_primary)
                .accessibility_hidden(true)
                .paint(
                    Rect::new(
                        content.origin.x - scroll_x,
                        content.origin.y - scroll_y,
                        content_width.max(content.size.width),
                        content_height.max(content.size.height),
                    ),
                    context,
                );
        }

        if focused && selection.is_none() {
            let (line_index, _) = line_and_column(&value, cursor, &lines);
            let line = &lines[line_index];
            let prefix = &value[line.start..cursor];
            let prefix_width = self.text(prefix)
                .measure_unbounded_with_typography(context.text_measurer, context.typography).width;
            let now = Instant::now();
            let (visible, next_redraw) = caret_state(&self.interaction, now);
            context.request_redraw_in_at(bounds, next_redraw);
            if visible {
                context.display_list.push(DrawCommand::FillRoundedRect {
                    rect: Rect::new(
                        content.origin.x + prefix_width - scroll_x,
                        content.origin.y + line_index as f32 * line_height - scroll_y,
                        context.theme.text_field.caret_width,
                        line_height,
                    ),
                    radius: context.theme.text_field.caret_width / 2.0,
                    color: context.theme.text_field.caret,
                });
            }
        }
        context.display_list.push(DrawCommand::PopClip);
    }

    fn handle_event(&self, bounds: Rect, event: &ViewEvent, context: &mut EventContext<'_>) -> EventResult {
        match event {
            ViewEvent::PointerMoved { position } => {
                let hovered = bounds.contains(*position);
                let selecting = {
                    let mut inner = self.interaction.inner.borrow_mut();
                    inner.hovered = hovered;
                    inner.selecting && inner.focused
                };
                if hovered {
                    context.set_cursor(CursorIcon::Text);
                }
                if selecting {
                    let cursor = self.index_at_point(*position, bounds, context);
                    self.interaction.inner.borrow_mut().cursor = cursor;
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            ViewEvent::PointerPressed { position, button: PointerButton::Primary } if bounds.contains(*position) => {
                let cursor = self.index_at_point(*position, bounds, context);
                let mut inner = self.interaction.inner.borrow_mut();
                inner.focused = self.enabled;
                inner.cursor = cursor;
                inner.selection_anchor = Some(cursor);
                inner.selecting = true;
                inner.caret_blink_origin = Some(Instant::now());
                drop(inner);
                context.request_keyboard_focus(bounds);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerReleased { button: PointerButton::Primary, .. } => {
                let mut inner = self.interaction.inner.borrow_mut();
                if !inner.selecting { return EventResult::Ignored; }
                inner.selecting = false;
                if inner.selection_anchor == Some(inner.cursor) { inner.selection_anchor = None; }
                drop(inner);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerFocusRequested { position } => {
                let focused = bounds.contains(*position) && self.enabled;
                let mut inner = self.interaction.inner.borrow_mut();
                inner.focused = focused;
                if !focused { inner.selecting = false; inner.caret_blink_origin = None; }
                EventResult::Ignored
            }
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let focused = target.is_some_and(|target| target == bounds) && self.enabled;
                self.interaction.inner.borrow_mut().focused = focused;
                EventResult::Ignored
            }
            ViewEvent::FocusChanged { focused: false } => {
                let mut inner = self.interaction.inner.borrow_mut();
                inner.focused = false;
                inner.selecting = false;
                inner.caret_blink_origin = None;
                EventResult::Ignored
            }
            ViewEvent::Scroll { position, delta_x, delta_y } if bounds.contains(*position) => {
                let mut inner = self.interaction.inner.borrow_mut();
                inner.scroll_x = (inner.scroll_x - delta_x).max(0.0);
                inner.scroll_y = (inner.scroll_y - delta_y).max(0.0);
                drop(inner);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::TextInput { text } if self.interaction.is_focused() => {
                let inserted: String = text.chars().filter(|character| *character == '\t' || !character.is_control()).collect();
                if !inserted.is_empty() && self.interaction.edit(|inner| Self::replace_selection(inner, &inserted)) {
                    self.synchronize(); self.interaction.reset_caret(); context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::Enter, .. } if self.interaction.is_focused() => {
                if self.interaction.edit(|inner| Self::replace_selection(inner, "\n")) { self.synchronize(); }
                self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::Tab, .. } if self.interaction.is_focused() => {
                if self.interaction.edit(|inner| Self::replace_selection(inner, "    ")) { self.synchronize(); }
                self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::Character(character), modifiers } if self.interaction.is_focused() && modifiers.shortcut() && (*character == 'z' || *character == 'Z') => {
                let changed = if modifiers.shift() { self.interaction.redo() } else { self.interaction.undo() };
                if changed { self.synchronize(); context.request_redraw_in(bounds); }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::Character(character), modifiers } if self.interaction.is_focused() && modifiers.shortcut() && (*character == 'y' || *character == 'Y') => {
                if self.interaction.redo() { self.synchronize(); context.request_redraw_in(bounds); }
                EventResult::Consumed
            }
            ViewEvent::Backspace if self.interaction.is_focused() => {
                let changed = self.interaction.edit(|inner| {
                    if selection_range(inner).is_some() { return Self::replace_selection(inner, ""); }
                    let previous = previous_boundary(&inner.value, inner.cursor);
                    if previous == inner.cursor { false } else { inner.value.replace_range(previous..inner.cursor, ""); inner.cursor = previous; true }
                });
                if changed { self.synchronize(); self.interaction.reset_caret(); context.request_redraw_in(bounds); }
                EventResult::Consumed
            }
            ViewEvent::Delete if self.interaction.is_focused() => {
                let changed = self.interaction.edit(|inner| {
                    if selection_range(inner).is_some() { return Self::replace_selection(inner, ""); }
                    let next = next_boundary(&inner.value, inner.cursor);
                    if next == inner.cursor { false } else { inner.value.replace_range(inner.cursor..next, ""); true }
                });
                if changed { self.synchronize(); self.interaction.reset_caret(); context.request_redraw_in(bounds); }
                EventResult::Consumed
            }
            ViewEvent::ArrowLeft | ViewEvent::SelectLeft if self.interaction.is_focused() => {
                self.move_horizontal(false, matches!(event, ViewEvent::SelectLeft)); self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::ArrowRight | ViewEvent::SelectRight if self.interaction.is_focused() => {
                self.move_horizontal(true, matches!(event, ViewEvent::SelectRight)); self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::ArrowUp, modifiers } if self.interaction.is_focused() => {
                self.move_vertical(false, modifiers.shift()); self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::ArrowDown, modifiers } if self.interaction.is_focused() => {
                self.move_vertical(true, modifiers.shift()); self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::Home | ViewEvent::SelectHome if self.interaction.is_focused() => {
                self.move_line_edge(false, matches!(event, ViewEvent::SelectHome)); self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::End | ViewEvent::SelectEnd if self.interaction.is_focused() => {
                self.move_line_edge(true, matches!(event, ViewEvent::SelectEnd)); self.interaction.reset_caret(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            ViewEvent::SelectAll if self.interaction.is_focused() => {
                self.select_all(); context.request_redraw_in(bounds); EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}

fn normalize_newlines(value: String) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn line_ranges(value: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if character == '\n' {
            ranges.push(start..index);
            start = index + 1;
        }
    }
    ranges.push(start..value.len());
    ranges
}

fn selection_range(inner: &TextEditorInteractionInner) -> Option<Range<usize>> {
    let anchor = inner.selection_anchor?.min(inner.value.len());
    let cursor = inner.cursor.min(inner.value.len());
    (anchor != cursor).then_some(anchor.min(cursor)..anchor.max(cursor))
}

fn clamp_boundary(value: &str, index: usize) -> usize {
    let mut index = index.min(value.len());
    while index > 0 && !value.is_char_boundary(index) { index -= 1; }
    index
}

fn previous_boundary(value: &str, cursor: usize) -> usize {
    value[..clamp_boundary(value, cursor)].char_indices().next_back().map(|(index, _)| index).unwrap_or(0)
}

fn next_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_boundary(value, cursor);
    value[cursor..].chars().next().map(|character| cursor + character.len_utf8()).unwrap_or(cursor)
}

fn line_and_column(value: &str, cursor: usize, lines: &[Range<usize>]) -> (usize, usize) {
    let cursor = clamp_boundary(value, cursor);
    let line = lines.iter().position(|range| cursor <= range.end).unwrap_or(lines.len().saturating_sub(1));
    let column = value[lines[line].start..cursor].chars().count();
    (line, column)
}

fn index_for_column(value: &str, range: &Range<usize>, column: usize) -> usize {
    value[range.clone()].char_indices().nth(column).map(|(index, _)| range.start + index).unwrap_or(range.end)
}

fn caret_state(interaction: &TextEditorInteractionState, now: Instant) -> (bool, Instant) {
    let mut inner = interaction.inner.borrow_mut();
    let origin = *inner.caret_blink_origin.get_or_insert(now);
    let elapsed = now.saturating_duration_since(origin).as_millis();
    let interval = CARET_BLINK_INTERVAL.as_millis();
    let visible = (elapsed / interval).is_multiple_of(2);
    (visible, now + Duration::from_millis((interval - elapsed % interval) as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_navigation_uses_character_columns() {
        let value = "abc\n日本語\nx";
        let lines = line_ranges(value);
        assert_eq!(line_and_column(value, 2, &lines), (0, 2));
        assert_eq!(index_for_column(value, &lines[1], 2), 10);
        assert_eq!(index_for_column(value, &lines[2], 8), value.len());
    }

    #[test]
    fn undo_and_redo_restore_document_and_cursor() {
        let state = TextEditorInteractionState::new();
        state.set_value("one");
        assert!(state.edit(|inner| TextEditor::replace_selection(inner, " two")));
        assert_eq!(state.value(), "one two");
        assert!(state.undo());
        assert_eq!(state.value(), "one");
        assert!(state.redo());
        assert_eq!(state.value(), "one two");
    }

    #[test]
    fn normalizes_platform_newlines() {
        assert_eq!(normalize_newlines("a\r\nb\rc".into()), "a\nb\nc");
    }
}
