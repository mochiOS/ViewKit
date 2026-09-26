//! Multi-line editable text surface.

use super::{BorderStyle, Rectangle, RectangleColor, Text};
use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::command::{CommandStatus, standard as commands};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::platform::clipboard::{
    set_text as set_system_clipboard_text, text as system_clipboard_text,
};
use crate::platform::{CursorIcon, Key, PointerButton};
use crate::state::Binding;
use crate::theme::{CornerRadius, ScrollBarTokens, ShadowStyle};
use crate::typography::TextRole;
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const HISTORY_GROUP_INTERVAL: Duration = Duration::from_millis(750);
const MAX_HISTORY: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditKind {
    Insert,
    Delete,
    Discrete,
}

#[derive(Clone, Debug, PartialEq)]
struct EditorSnapshot {
    value: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

#[derive(Debug)]
struct TextEditorInteractionInner {
    value: String,
    revision: u64,
    line_count: usize,
    character_count: usize,
    cursor: usize,
    selection_anchor: Option<usize>,
    selecting: bool,
    focused: bool,
    hovered: bool,
    enabled: bool,
    initialized: bool,
    scroll_x: f32,
    scroll_y: f32,
    reveal_caret: bool,
    vertical_scroll_drag: Option<f32>,
    horizontal_scroll_drag: Option<f32>,
    cached_layout_value: String,
    cached_layout_monospaced: bool,
    cached_layout_line_height: f32,
    cached_layout_wrap_width: Option<f32>,
    cached_line_ranges: Vec<Range<usize>>,
    cached_line_widths: Vec<f32>,
    caret_blink_origin: Option<Instant>,
    undo: Vec<EditorSnapshot>,
    redo: Vec<EditorSnapshot>,
    last_edit_kind: Option<EditKind>,
    last_edit_at: Option<Instant>,
}

impl Default for TextEditorInteractionInner {
    fn default() -> Self {
        Self {
            value: String::new(),
            revision: 0,
            line_count: 1,
            character_count: 0,
            cursor: 0,
            selection_anchor: None,
            selecting: false,
            focused: false,
            hovered: false,
            enabled: true,
            initialized: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            reveal_caret: true,
            vertical_scroll_drag: None,
            horizontal_scroll_drag: None,
            cached_layout_value: String::new(),
            cached_layout_monospaced: false,
            cached_layout_line_height: 0.0,
            cached_layout_wrap_width: None,
            cached_line_ranges: Vec::new(),
            cached_line_widths: Vec::new(),
            caret_blink_origin: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit_kind: None,
            last_edit_at: None,
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
        inner.revision = inner.revision.wrapping_add(1);
        update_statistics(&mut inner);
        inner.cursor = inner.value.len();
        inner.selection_anchor = None;
        inner.selecting = false;
        inner.scroll_x = 0.0;
        inner.scroll_y = 0.0;
        inner.reveal_caret = true;
        inner.vertical_scroll_drag = None;
        inner.horizontal_scroll_drag = None;
        inner.undo.clear();
        inner.redo.clear();
        inner.last_edit_kind = None;
        inner.last_edit_at = None;
        inner.initialized = true;
    }

    pub fn is_focused(&self) -> bool {
        self.inner.borrow().focused
    }

    /// Gives the editor initial keyboard focus before the first pointer event.
    /// Later focus changes from the window system still take precedence.
    pub fn focus(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.focused = true;
        inner.caret_blink_origin = Some(Instant::now());
        inner.reveal_caret = true;
    }

    pub fn revision(&self) -> u64 {
        self.inner.borrow().revision
    }

    /// Returns the logical line count and Unicode scalar-value count without
    /// cloning the document.
    pub fn statistics(&self) -> (usize, usize) {
        let inner = self.inner.borrow();
        (inner.line_count, inner.character_count)
    }

    pub fn can_undo(&self) -> bool {
        !self.inner.borrow().undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.inner.borrow().redo.is_empty()
    }

    /// Selects a matching range and returns its one-based index and the total
    /// number of matches. `from_start` restarts a search after the query has
    /// changed; subsequent calls continue from the current selection and wrap.
    pub fn find_match(
        &self,
        query: &str,
        backwards: bool,
        from_start: bool,
    ) -> Option<(usize, usize)> {
        if query.is_empty() {
            return None;
        }

        let mut inner = self.inner.borrow_mut();
        let matches: Vec<Range<usize>> = inner
            .value
            .match_indices(query)
            .map(|(start, value)| start..start + value.len())
            .collect();
        if matches.is_empty() {
            inner.selection_anchor = None;
            return None;
        }

        let selected = selection_range(&inner);
        let index = if from_start {
            if backwards { matches.len() - 1 } else { 0 }
        } else if backwards {
            let before = selected.as_ref().map_or(inner.cursor, |range| range.start);
            matches
                .iter()
                .rposition(|range| range.end <= before)
                .unwrap_or(matches.len() - 1)
        } else {
            let after = selected.as_ref().map_or(inner.cursor, |range| range.end);
            matches
                .iter()
                .position(|range| range.start >= after)
                .unwrap_or(0)
        };
        let range = matches[index].clone();
        inner.selection_anchor = Some(range.start);
        inner.cursor = range.end;
        inner.selecting = false;
        inner.reveal_caret = true;
        inner.caret_blink_origin = Some(Instant::now());
        Some((index + 1, matches.len()))
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
        inner.revision = inner.revision.wrapping_add(1);
        update_statistics(inner);
        inner.cursor = snapshot.cursor.min(inner.value.len());
        inner.selection_anchor = snapshot.selection_anchor;
        inner.selecting = false;
        inner.reveal_caret = true;
        inner.last_edit_kind = None;
        inner.last_edit_at = None;
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
        self.edit_with_kind(EditKind::Discrete, operation)
    }

    fn edit_with_kind(
        &self,
        kind: EditKind,
        operation: impl FnOnce(&mut TextEditorInteractionInner) -> bool,
    ) -> bool {
        let mut inner = self.inner.borrow_mut();
        let before = Self::snapshot(&inner);
        if !operation(&mut inner) {
            return false;
        }
        if Self::snapshot(&inner) == before {
            return false;
        }
        if inner.value != before.value {
            inner.revision = inner.revision.wrapping_add(1);
            update_statistics(&mut inner);
            let now = Instant::now();
            let grouped = kind != EditKind::Discrete
                && inner.last_edit_kind == Some(kind)
                && inner.last_edit_at.is_some_and(|previous| {
                    now.saturating_duration_since(previous) <= HISTORY_GROUP_INTERVAL
                });
            if !grouped && inner.undo.last() != Some(&before) {
                inner.undo.push(before);
                if inner.undo.len() > MAX_HISTORY {
                    inner.undo.remove(0);
                }
            }
            inner.redo.clear();
            inner.last_edit_kind = Some(kind);
            inner.last_edit_at = Some(now);
        }
        inner.reveal_caret = true;
        true
    }

    fn reset_caret(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.caret_blink_origin = Some(Instant::now());
        inner.reveal_caret = true;
    }
}

pub struct TextEditor {
    interaction: TextEditorInteractionState,
    binding: Option<Binding<String>>,
    placeholder: String,
    enabled: bool,
    monospaced: bool,
    font_size: Option<f32>,
    line_wrap: bool,
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
            font_size: None,
            line_wrap: false,
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
            font_size: None,
            line_wrap: false,
            bordered: false,
            on_change: None,
        }
    }

    pub fn value(self, value: impl Into<String>) -> Self {
        self.interaction
            .initialize(normalize_newlines(value.into()));
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

    /// Sets the editor text size in logical pixels.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = (font_size.is_finite() && font_size > 0.0).then_some(font_size);
        self
    }

    /// Wraps long logical lines to the available editor width.
    pub fn line_wrap(mut self, line_wrap: bool) -> Self {
        self.line_wrap = line_wrap;
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
        let mut text = Text::styled(value, TextRole::Body).cache_layout(false);
        if let Some(font_size) = self.font_size {
            text = text.font_size(font_size).line_height(font_size * 1.4);
        }
        if self.monospaced {
            text.monospaced()
        } else {
            text
        }
    }

    fn line_height(&self, context: &PaintContext<'_>) -> f32 {
        self.font_size
            .map(|font_size| font_size * 1.4)
            .unwrap_or(context.typography.style(TextRole::Body).line_height)
            * context.text_measurer.font_scale()
    }

    fn event_line_height(&self, context: &EventContext<'_>) -> f32 {
        self.font_size
            .map(|font_size| font_size * 1.4)
            .unwrap_or(context.typography.style(TextRole::Body).line_height)
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
        let padding = context.theme.spacing.large;
        let line_height = self.event_line_height(context).max(1.0);
        let content = Self::content_bounds(bounds, padding);
        let mut inner = self.interaction.inner.borrow_mut();
        refresh_line_width_cache(
            &mut inner,
            self.monospaced,
            line_height,
            self.line_wrap.then_some(content.size.width),
            |line| {
                self.text(line)
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width
            },
        );
        let target_line = ((point.y - bounds.origin.y - padding + inner.scroll_y) / line_height)
            .floor()
            .max(0.0) as usize;
        let lines = if self.line_wrap && !inner.cached_line_ranges.is_empty() {
            inner.cached_line_ranges.clone()
        } else {
            line_ranges(&inner.value)
        };
        let range = lines
            .get(target_line)
            .or_else(|| lines.last())
            .cloned()
            .unwrap_or(0..0);
        let line = &inner.value[range.clone()];
        let target_x = (point.x - bounds.origin.x - padding
            + if self.line_wrap { 0.0 } else { inner.scroll_x })
        .max(0.0);
        let mut previous = range.start;
        let mut previous_width = 0.0;
        for (relative, character) in line.char_indices() {
            let next = relative + character.len_utf8();
            let width = self
                .text(&line[..next])
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
        inner.reveal_caret = true;
        inner.last_edit_kind = None;
        inner.last_edit_at = None;
        changed
    }

    fn move_horizontal(&self, right: bool, extend: bool) -> bool {
        let inner = self.interaction.inner.borrow();
        let cursor = inner.cursor;
        if !extend {
            if let Some(selection) = selection_range(&inner) {
                let target = if right {
                    selection.end
                } else {
                    selection.start
                };
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
        self.move_vertical_by(if down { 1 } else { -1 }, extend)
    }

    fn move_vertical_by(&self, amount: isize, extend: bool) -> bool {
        let inner = self.interaction.inner.borrow();
        let lines = if self.line_wrap && !inner.cached_line_ranges.is_empty() {
            inner.cached_line_ranges.clone()
        } else {
            line_ranges(&inner.value)
        };
        let (line_index, column) = line_and_column(&inner.value, inner.cursor, &lines);
        let target_line = line_index
            .saturating_add_signed(amount)
            .min(lines.len().saturating_sub(1));
        let target = index_for_column(&inner.value, &lines[target_line], column);
        drop(inner);
        self.set_cursor(target, extend)
    }

    fn move_line_edge(&self, end: bool, extend: bool) -> bool {
        let inner = self.interaction.inner.borrow();
        let lines = if self.line_wrap && !inner.cached_line_ranges.is_empty() {
            inner.cached_line_ranges.clone()
        } else {
            line_ranges(&inner.value)
        };
        let (line, _) = line_and_column(&inner.value, inner.cursor, &lines);
        let target = if end {
            lines[line].end
        } else {
            lines[line].start
        };
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
        inner.last_edit_kind = None;
        inner.last_edit_at = None;
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

    fn event_document_size(&self, bounds: Rect, context: &mut EventContext<'_>) -> Size {
        let line_height = self.event_line_height(context).max(1.0);
        let content = Self::content_bounds(bounds, context.theme.spacing.large);
        let mut inner = self.interaction.inner.borrow_mut();
        refresh_line_width_cache(
            &mut inner,
            self.monospaced,
            line_height,
            self.line_wrap.then_some(content.size.width),
            |line| {
                self.text(line)
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width
            },
        );
        let width = inner
            .cached_line_widths
            .iter()
            .copied()
            .fold(0.0_f32, f32::max)
            + context.theme.text_field.caret_width;
        let line_count = inner.cached_line_ranges.len();
        Size::new(width, line_count as f32 * line_height)
    }

    fn begin_scrollbar_drag(
        &self,
        position: Point,
        bounds: Rect,
        context: &mut EventContext<'_>,
    ) -> bool {
        let padding = context.theme.spacing.large;
        let viewport = Self::content_bounds(bounds, padding);
        let document = self.event_document_size(bounds, context);
        let offset = {
            let inner = self.interaction.inner.borrow();
            Point::new(inner.scroll_x, inner.scroll_y)
        };
        let geometry =
            editor_scrollbar_geometry(viewport, document, offset, context.theme.scrollbar);

        if let Some(vertical) = geometry.vertical
            && vertical.track.expanded(4.0).contains(position)
        {
            let grab = if vertical.thumb.contains(position) {
                position.y - vertical.thumb.origin.y
            } else {
                vertical.thumb.size.height / 2.0
            };
            let mut inner = self.interaction.inner.borrow_mut();
            inner.vertical_scroll_drag = Some(grab);
            inner.horizontal_scroll_drag = None;
            inner.reveal_caret = false;
            inner.scroll_y = scrollbar_offset(
                position.y,
                grab,
                vertical.track,
                vertical.thumb.size.height,
                vertical.maximum_offset,
            );
            return true;
        }

        if let Some(horizontal) = geometry.horizontal
            && horizontal.track.expanded(4.0).contains(position)
        {
            let grab = if horizontal.thumb.contains(position) {
                position.x - horizontal.thumb.origin.x
            } else {
                horizontal.thumb.size.width / 2.0
            };
            let mut inner = self.interaction.inner.borrow_mut();
            inner.horizontal_scroll_drag = Some(grab);
            inner.vertical_scroll_drag = None;
            inner.reveal_caret = false;
            inner.scroll_x = scrollbar_offset(
                position.x,
                grab,
                horizontal.track,
                horizontal.thumb.size.width,
                horizontal.maximum_offset,
            );
            return true;
        }

        false
    }

    fn update_scrollbar_drag(
        &self,
        position: Point,
        bounds: Rect,
        context: &mut EventContext<'_>,
    ) -> bool {
        let (vertical_grab, horizontal_grab, offset) = {
            let inner = self.interaction.inner.borrow();
            (
                inner.vertical_scroll_drag,
                inner.horizontal_scroll_drag,
                Point::new(inner.scroll_x, inner.scroll_y),
            )
        };
        if vertical_grab.is_none() && horizontal_grab.is_none() {
            return false;
        }

        let viewport = Self::content_bounds(bounds, context.theme.spacing.large);
        let document = self.event_document_size(bounds, context);
        let geometry =
            editor_scrollbar_geometry(viewport, document, offset, context.theme.scrollbar);
        let mut inner = self.interaction.inner.borrow_mut();
        if let (Some(grab), Some(vertical)) = (vertical_grab, geometry.vertical) {
            inner.scroll_y = scrollbar_offset(
                position.y,
                grab,
                vertical.track,
                vertical.thumb.size.height,
                vertical.maximum_offset,
            );
        }
        if let (Some(grab), Some(horizontal)) = (horizontal_grab, geometry.horizontal) {
            inner.scroll_x = scrollbar_offset(
                position.x,
                grab,
                horizontal.track,
                horizontal.thumb.size.width,
                horizontal.maximum_offset,
            );
        }
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
        let can_undo = !inner.undo.is_empty();
        let can_redo = !inner.redo.is_empty();
        let line_height = self.line_height(context).max(1.0);
        refresh_line_width_cache(
            &mut inner,
            self.monospaced,
            line_height,
            self.line_wrap.then_some(content.size.width),
            |line| {
                self.text(line)
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width
            },
        );
        let lines = inner.cached_line_ranges.clone();
        let line_widths = inner.cached_line_widths.clone();
        let content_width = line_widths.iter().copied().fold(0.0_f32, f32::max)
            + context.theme.text_field.caret_width;
        let content_height = lines.len() as f32 * line_height;
        let maximum_x = if self.line_wrap {
            0.0
        } else {
            (content_width - content.size.width).max(0.0)
        };
        let maximum_y = (content_height - content.size.height).max(0.0);
        inner.scroll_x = inner.scroll_x.clamp(0.0, maximum_x);
        inner.scroll_y = inner.scroll_y.clamp(0.0, maximum_y);

        let (cursor_line, _) = line_and_column(&value, cursor, &lines);
        let cursor_prefix = &value[lines[cursor_line].start..cursor];
        let cursor_x = self
            .text(cursor_prefix)
            .measure_unbounded_with_typography(context.text_measurer, context.typography)
            .width;
        if inner.reveal_caret && focused {
            let horizontal_margin = context.theme.spacing.small;
            if cursor_x < inner.scroll_x {
                inner.scroll_x = cursor_x.max(0.0);
            } else if cursor_x + horizontal_margin > inner.scroll_x + content.size.width {
                inner.scroll_x = (cursor_x + horizontal_margin - content.size.width).min(maximum_x);
            }
            let cursor_y = cursor_line as f32 * line_height;
            if cursor_y < inner.scroll_y {
                inner.scroll_y = cursor_y;
            } else if cursor_y + line_height > inner.scroll_y + content.size.height {
                inner.scroll_y = (cursor_y + line_height - content.size.height).min(maximum_y);
            }
            inner.reveal_caret = false;
        }
        let scroll_x = inner.scroll_x;
        let scroll_y = inner.scroll_y;
        drop(inner);

        context.record_command_status(CommandStatus::new(
            commands::UNDO,
            bounds,
            focused && can_undo,
        ));
        context.record_command_status(CommandStatus::new(
            commands::REDO,
            bounds,
            focused && can_redo,
        ));
        let has_selection = selection.is_some();
        context.record_command_status(CommandStatus::new(
            commands::COPY,
            bounds,
            focused && has_selection,
        ));
        context.record_command_status(CommandStatus::new(
            commands::CUT,
            bounds,
            focused && has_selection,
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

        let first_visible_line = (scroll_y / line_height).floor() as usize;
        let last_visible_line = ((scroll_y + content.size.height) / line_height).ceil() as usize;
        let visible_lines = first_visible_line.min(lines.len())..last_visible_line.min(lines.len());

        let mut node = AccessibilityNode::new(AccessibilityRole::TextField, bounds);
        node.label = Some(if self.placeholder.is_empty() {
            "Document".into()
        } else {
            self.placeholder.clone()
        });
        node.value = Some(value.clone());
        node.enabled = self.enabled;
        node.focusable = true;
        node.focused = focused;
        context.record_accessibility(node);

        Rectangle::new()
            .color(RectangleColor::Custom(context.theme.colors.surface))
            .radius(if self.bordered {
                CornerRadius::Small
            } else {
                CornerRadius::None
            })
            .shadow(ShadowStyle::None)
            .border(if self.bordered {
                BorderStyle::custom(
                    context.theme.text_field.border,
                    context.theme.text_field.stroke_width,
                )
            } else {
                BorderStyle::None
            })
            .paint(bounds, context);

        context
            .display_list
            .push(DrawCommand::PushClip { rect: content });
        if let Some(selection) = selection.as_ref() {
            for line_index in visible_lines.clone() {
                let range = &lines[line_index];
                let start = selection.start.max(range.start);
                let end = selection.end.min(range.end);
                if start >= end && !(selection.end > range.end && selection.start <= range.end) {
                    continue;
                }
                let line = &value[range.clone()];
                let start_width = self
                    .text(&line[..start.saturating_sub(range.start)])
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;
                let end_width = self
                    .text(&line[..end.saturating_sub(range.start)])
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;
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
            for line_index in visible_lines.clone() {
                let range = &lines[line_index];
                self.text(value[range.clone()].to_owned())
                    .color(context.theme.colors.text_primary)
                    .accessibility_hidden(true)
                    .paint(
                        Rect::new(
                            content.origin.x - scroll_x,
                            content.origin.y + line_index as f32 * line_height - scroll_y,
                            (line_widths[line_index] + context.theme.text_field.caret_width)
                                .max(content.size.width),
                            line_height,
                        ),
                        context,
                    );
            }
        }

        if focused && selection.is_none() {
            let now = Instant::now();
            let (visible, next_redraw) = caret_state(&self.interaction, now);
            context.request_redraw_in_at(bounds, next_redraw);
            if visible {
                context.display_list.push(DrawCommand::FillRoundedRect {
                    rect: Rect::new(
                        content.origin.x + cursor_x - scroll_x,
                        content.origin.y + cursor_line as f32 * line_height - scroll_y,
                        context.theme.text_field.caret_width,
                        line_height,
                    ),
                    radius: context.theme.text_field.caret_width / 2.0,
                    color: context.theme.text_field.caret,
                });
            }
        }
        context.display_list.push(DrawCommand::PopClip);

        paint_editor_scrollbars(
            content,
            Size::new(content_width, content_height),
            Point::new(scroll_x, scroll_y),
            context.theme.scrollbar,
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        match event {
            ViewEvent::PointerMoved { position } => {
                if self.update_scrollbar_drag(*position, bounds, context) {
                    context.request_redraw_in(bounds);
                    return EventResult::Consumed;
                }
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
                    let content = Self::content_bounds(bounds, context.theme.spacing.large);
                    {
                        let mut inner = self.interaction.inner.borrow_mut();
                        if position.y < content.origin.y {
                            inner.scroll_y = (inner.scroll_y
                                - (content.origin.y - position.y).min(32.0))
                            .max(0.0);
                        } else if position.y > content.origin.y + content.size.height {
                            inner.scroll_y +=
                                (position.y - content.origin.y - content.size.height).min(32.0);
                        }
                        if position.x < content.origin.x {
                            inner.scroll_x = (inner.scroll_x
                                - (content.origin.x - position.x).min(32.0))
                            .max(0.0);
                        } else if position.x > content.origin.x + content.size.width {
                            inner.scroll_x +=
                                (position.x - content.origin.x - content.size.width).min(32.0);
                        }
                        inner.reveal_caret = false;
                    }
                    let cursor = self.index_at_point(*position, bounds, context);
                    self.interaction.inner.borrow_mut().cursor = cursor;
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(*position) => {
                if self.begin_scrollbar_drag(*position, bounds, context) {
                    context.request_redraw_in(bounds);
                    return EventResult::Consumed;
                }
                let cursor = self.index_at_point(*position, bounds, context);
                let mut inner = self.interaction.inner.borrow_mut();
                inner.focused = self.enabled;
                inner.cursor = cursor;
                inner.selection_anchor = Some(cursor);
                inner.selecting = true;
                inner.reveal_caret = true;
                inner.last_edit_kind = None;
                inner.last_edit_at = None;
                inner.caret_blink_origin = Some(Instant::now());
                drop(inner);
                context.request_keyboard_focus(bounds);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } => {
                let mut inner = self.interaction.inner.borrow_mut();
                let ended_scroll_drag = inner.vertical_scroll_drag.take().is_some()
                    || inner.horizontal_scroll_drag.take().is_some();
                if ended_scroll_drag {
                    drop(inner);
                    context.request_redraw_in(bounds);
                    return EventResult::Consumed;
                }
                if !inner.selecting {
                    return EventResult::Ignored;
                }
                inner.selecting = false;
                if inner.selection_anchor == Some(inner.cursor) {
                    inner.selection_anchor = None;
                }
                drop(inner);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerFocusRequested { position } => {
                let focused = bounds.contains(*position) && self.enabled;
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != focused;
                inner.focused = focused;
                if !focused {
                    inner.selecting = false;
                    inner.caret_blink_origin = None;
                }
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let focused = target.is_some_and(|target| target == bounds) && self.enabled;
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != focused;
                inner.focused = focused;
                if focused {
                    inner.caret_blink_origin = Some(Instant::now());
                } else {
                    inner.selecting = false;
                    inner.caret_blink_origin = None;
                }
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            ViewEvent::FocusChanged { focused: false } => {
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused || inner.selecting;
                inner.focused = false;
                inner.selecting = false;
                inner.caret_blink_origin = None;
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            ViewEvent::Scroll {
                position,
                delta_x,
                delta_y,
            } if bounds.contains(*position) => {
                let mut inner = self.interaction.inner.borrow_mut();
                inner.scroll_x = (inner.scroll_x + delta_x).max(0.0);
                inner.scroll_y = (inner.scroll_y + delta_y).max(0.0);
                inner.reveal_caret = false;
                drop(inner);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::TextInput { text } if self.interaction.is_focused() => {
                let inserted: String = text
                    .chars()
                    .filter(|character| *character == '\t' || !character.is_control())
                    .collect();
                if !inserted.is_empty()
                    && self.interaction.edit_with_kind(EditKind::Insert, |inner| {
                        Self::replace_selection(inner, &inserted)
                    })
                {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::Enter, ..
            } if self.interaction.is_focused() => {
                if self
                    .interaction
                    .edit(|inner| Self::replace_selection(inner, "\n"))
                {
                    self.synchronize();
                }
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::KeyPressed { key: Key::Tab, .. } if self.interaction.is_focused() => {
                if self
                    .interaction
                    .edit(|inner| Self::replace_selection(inner, "    "))
                {
                    self.synchronize();
                }
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'z' || *character == 'Z') =>
            {
                let changed = if modifiers.shift() {
                    self.interaction.redo()
                } else {
                    self.interaction.undo()
                };
                if changed {
                    self.synchronize();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'y' || *character == 'Y') =>
            {
                if self.interaction.redo() {
                    self.synchronize();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'c' || *character == 'C') =>
            {
                let selected = {
                    let inner = self.interaction.inner.borrow();
                    selection_range(&inner).map(|range| inner.value[range].to_owned())
                };
                if let Some(selected) = selected {
                    let _ = set_system_clipboard_text(&selected);
                }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'x' || *character == 'X') =>
            {
                let selected = {
                    let inner = self.interaction.inner.borrow();
                    selection_range(&inner).map(|range| inner.value[range].to_owned())
                };
                if let Some(selected) = selected
                    && set_system_clipboard_text(&selected)
                    && self
                        .interaction
                        .edit(|inner| Self::replace_selection(inner, ""))
                {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'v' || *character == 'V') =>
            {
                if let Some(pasted) = system_clipboard_text()
                    && !pasted.is_empty()
                    && self
                        .interaction
                        .edit(|inner| Self::replace_selection(inner, &normalize_newlines(pasted)))
                {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::UNDO =>
            {
                if self.interaction.undo() {
                    self.synchronize();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::REDO =>
            {
                if self.interaction.redo() {
                    self.synchronize();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::COPY =>
            {
                let selected = {
                    let inner = self.interaction.inner.borrow();
                    selection_range(&inner).map(|range| inner.value[range].to_owned())
                };
                if let Some(selected) = selected {
                    let _ = set_system_clipboard_text(&selected);
                }
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::CUT =>
            {
                let selected = {
                    let inner = self.interaction.inner.borrow();
                    selection_range(&inner).map(|range| inner.value[range].to_owned())
                };
                if let Some(selected) = selected
                    && set_system_clipboard_text(&selected)
                    && self
                        .interaction
                        .edit(|inner| Self::replace_selection(inner, ""))
                {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::PASTE =>
            {
                if let Some(pasted) = system_clipboard_text()
                    && !pasted.is_empty()
                    && self
                        .interaction
                        .edit(|inner| Self::replace_selection(inner, &normalize_newlines(pasted)))
                {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::SELECT_ALL =>
            {
                self.select_all();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::Command { command, .. }
                if self.interaction.is_focused() && *command == commands::DELETE =>
            {
                let changed = self.interaction.edit_with_kind(EditKind::Delete, |inner| {
                    if selection_range(inner).is_some() {
                        return Self::replace_selection(inner, "");
                    }
                    let next = next_boundary(&inner.value, inner.cursor);
                    if next == inner.cursor {
                        false
                    } else {
                        inner.value.replace_range(inner.cursor..next, "");
                        true
                    }
                });
                if changed {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Backspace if self.interaction.is_focused() => {
                let changed = self.interaction.edit_with_kind(EditKind::Delete, |inner| {
                    if selection_range(inner).is_some() {
                        return Self::replace_selection(inner, "");
                    }
                    let previous = previous_boundary(&inner.value, inner.cursor);
                    if previous == inner.cursor {
                        false
                    } else {
                        inner.value.replace_range(previous..inner.cursor, "");
                        inner.cursor = previous;
                        true
                    }
                });
                if changed {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::Delete if self.interaction.is_focused() => {
                let changed = self.interaction.edit_with_kind(EditKind::Delete, |inner| {
                    if selection_range(inner).is_some() {
                        return Self::replace_selection(inner, "");
                    }
                    let next = next_boundary(&inner.value, inner.cursor);
                    if next == inner.cursor {
                        false
                    } else {
                        inner.value.replace_range(inner.cursor..next, "");
                        true
                    }
                });
                if changed {
                    self.synchronize();
                    self.interaction.reset_caret();
                    context.request_redraw_in(bounds);
                }
                EventResult::Consumed
            }
            ViewEvent::ArrowLeft | ViewEvent::SelectLeft if self.interaction.is_focused() => {
                self.move_horizontal(false, matches!(event, ViewEvent::SelectLeft));
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::ArrowRight | ViewEvent::SelectRight if self.interaction.is_focused() => {
                self.move_horizontal(true, matches!(event, ViewEvent::SelectRight));
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::ArrowUp,
                modifiers,
            } if self.interaction.is_focused() => {
                self.move_vertical(false, modifiers.shift());
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::ArrowDown,
                modifiers,
            } if self.interaction.is_focused() => {
                self.move_vertical(true, modifiers.shift());
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::PageUp,
                modifiers,
            } if self.interaction.is_focused() => {
                let page = (bounds.size.height / self.event_line_height(context).max(1.0))
                    .floor()
                    .max(1.0) as isize;
                self.move_vertical_by(-page, modifiers.shift());
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::KeyPressed {
                key: Key::PageDown,
                modifiers,
            } if self.interaction.is_focused() => {
                let page = (bounds.size.height / self.event_line_height(context).max(1.0))
                    .floor()
                    .max(1.0) as isize;
                self.move_vertical_by(page, modifiers.shift());
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::Home | ViewEvent::SelectHome if self.interaction.is_focused() => {
                self.move_line_edge(false, matches!(event, ViewEvent::SelectHome));
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::End | ViewEvent::SelectEnd if self.interaction.is_focused() => {
                self.move_line_edge(true, matches!(event, ViewEvent::SelectEnd));
                self.interaction.reset_caret();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::SelectAll if self.interaction.is_focused() => {
                self.select_all();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}

#[derive(Clone, Copy)]
struct EditorScrollbar {
    track: Rect,
    thumb: Rect,
    maximum_offset: f32,
}

#[derive(Clone, Copy, Default)]
struct EditorScrollbarGeometry {
    vertical: Option<EditorScrollbar>,
    horizontal: Option<EditorScrollbar>,
}

fn editor_scrollbar_geometry(
    viewport: Rect,
    document: Size,
    offset: Point,
    tokens: ScrollBarTokens,
) -> EditorScrollbarGeometry {
    let vertical_visible = document.height > viewport.size.height;
    let horizontal_visible = document.width > viewport.size.width;
    let thickness = tokens.thickness.max(0.0);
    let inset = tokens.inset.max(0.0);
    let length_inset = tokens.length_inset.max(0.0);
    if thickness <= 0.0 {
        return EditorScrollbarGeometry::default();
    }

    let vertical = vertical_visible.then(|| {
        let reserved_bottom = if horizontal_visible {
            thickness + inset
        } else {
            0.0
        };
        let track_length =
            (viewport.size.height - inset * 2.0 - length_inset * 2.0 - reserved_bottom).max(0.0);
        let track = Rect::new(
            viewport.origin.x + viewport.size.width - inset - thickness - tokens.horizontal_offset,
            viewport.origin.y + inset + length_inset,
            thickness,
            track_length,
        );
        let thumb_length = scrollbar_thumb_length(
            track_length,
            viewport.size.height,
            document.height,
            tokens.minimum_thumb_length,
        );
        let maximum_offset = (document.height - viewport.size.height).max(0.0);
        let progress = if maximum_offset > 0.0 {
            (offset.y / maximum_offset).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let thumb = Rect::new(
            track.origin.x,
            track.origin.y + (track_length - thumb_length).max(0.0) * progress,
            thickness,
            thumb_length,
        );
        EditorScrollbar {
            track,
            thumb,
            maximum_offset,
        }
    });

    let horizontal = horizontal_visible.then(|| {
        let reserved_right = if vertical_visible {
            thickness + inset
        } else {
            0.0
        };
        let track_length = (viewport.size.width - inset * 2.0 - reserved_right).max(0.0);
        let track = Rect::new(
            viewport.origin.x + inset,
            viewport.origin.y + viewport.size.height - inset - thickness,
            track_length,
            thickness,
        );
        let thumb_length = scrollbar_thumb_length(
            track_length,
            viewport.size.width,
            document.width,
            tokens.minimum_thumb_length,
        );
        let maximum_offset = (document.width - viewport.size.width).max(0.0);
        let progress = if maximum_offset > 0.0 {
            (offset.x / maximum_offset).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let thumb = Rect::new(
            track.origin.x + (track_length - thumb_length).max(0.0) * progress,
            track.origin.y,
            thumb_length,
            thickness,
        );
        EditorScrollbar {
            track,
            thumb,
            maximum_offset,
        }
    });

    EditorScrollbarGeometry {
        vertical,
        horizontal,
    }
}

fn scrollbar_thumb_length(track: f32, viewport: f32, document: f32, minimum: f32) -> f32 {
    if track <= 0.0 || document <= viewport || document <= 0.0 {
        return track.max(0.0);
    }
    (track * viewport / document).clamp(minimum.max(0.0).min(track), track)
}

fn scrollbar_offset(
    pointer: f32,
    grab: f32,
    track: Rect,
    thumb_length: f32,
    maximum_offset: f32,
) -> f32 {
    let track_start = if track.size.width > track.size.height {
        track.origin.x
    } else {
        track.origin.y
    };
    let track_length = if track.size.width > track.size.height {
        track.size.width
    } else {
        track.size.height
    };
    let travel = (track_length - thumb_length).max(0.0);
    if travel <= 0.0 || maximum_offset <= 0.0 {
        return 0.0;
    }
    ((pointer - track_start - grab) / travel).clamp(0.0, 1.0) * maximum_offset
}

fn paint_editor_scrollbars(
    viewport: Rect,
    document: Size,
    offset: Point,
    tokens: ScrollBarTokens,
    context: &mut PaintContext<'_>,
) {
    let geometry = editor_scrollbar_geometry(viewport, document, offset, tokens);
    for scrollbar in [geometry.vertical, geometry.horizontal]
        .into_iter()
        .flatten()
    {
        context.display_list.push(DrawCommand::FillRoundedRect {
            rect: scrollbar.track,
            radius: scrollbar.track.size.width.min(scrollbar.track.size.height) / 2.0,
            color: tokens.track_color,
        });
        context.display_list.push(DrawCommand::FillRoundedRect {
            rect: scrollbar.thumb,
            radius: scrollbar.thumb.size.width.min(scrollbar.thumb.size.height) / 2.0,
            color: tokens.thumb_color,
        });
    }
}

fn refresh_line_width_cache(
    inner: &mut TextEditorInteractionInner,
    monospaced: bool,
    line_height: f32,
    wrap_width: Option<f32>,
    mut measure: impl FnMut(String) -> f32,
) {
    let wrap_width = wrap_width.filter(|width| width.is_finite() && *width > 0.0);
    if inner.cached_layout_value == inner.value
        && inner.cached_layout_monospaced == monospaced
        && inner.cached_layout_line_height == line_height
        && inner.cached_layout_wrap_width == wrap_width
        && !inner.cached_line_ranges.is_empty()
        && !inner.cached_line_widths.is_empty()
    {
        return;
    }

    let value = inner.value.clone();
    let ranges = visual_line_ranges(&value, wrap_width, &mut measure);
    inner.cached_line_widths = ranges
        .iter()
        .map(|range| measure(value[range.clone()].to_owned()))
        .collect();
    inner.cached_line_ranges = ranges;
    inner.cached_layout_value = value;
    inner.cached_layout_monospaced = monospaced;
    inner.cached_layout_line_height = line_height;
    inner.cached_layout_wrap_width = wrap_width;
}

fn visual_line_ranges(
    value: &str,
    wrap_width: Option<f32>,
    measure: &mut impl FnMut(String) -> f32,
) -> Vec<Range<usize>> {
    let Some(wrap_width) = wrap_width else {
        return line_ranges(value);
    };
    let mut visual = Vec::new();
    for logical in line_ranges(value) {
        if logical.is_empty() {
            visual.push(logical);
            continue;
        }
        let line = &value[logical.clone()];
        if measure(line.to_owned()) <= wrap_width {
            visual.push(logical);
            continue;
        }

        let mut boundaries: Vec<usize> = line.char_indices().map(|(index, _)| index).collect();
        boundaries.push(line.len());
        let mut start_index = 0usize;
        while start_index + 1 < boundaries.len() {
            let mut low = start_index + 1;
            let mut high = boundaries.len() - 1;
            let mut fitting = low;
            while low <= high {
                let middle = low + (high - low) / 2;
                let width = measure(line[boundaries[start_index]..boundaries[middle]].to_owned());
                if width <= wrap_width || middle == start_index + 1 {
                    fitting = middle;
                    low = middle + 1;
                } else {
                    high = middle.saturating_sub(1);
                }
            }

            if fitting < boundaries.len() - 1 {
                let segment = &line[boundaries[start_index]..boundaries[fitting]];
                if let Some((break_index, character)) = segment
                    .char_indices()
                    .rev()
                    .find(|(_, character)| character.is_whitespace())
                {
                    let after_break = boundaries[start_index] + break_index + character.len_utf8();
                    if after_break > boundaries[start_index] {
                        fitting = boundaries.binary_search(&after_break).unwrap_or(fitting);
                    }
                }

                while fitting < boundaries.len() - 1
                    && line[boundaries[fitting]..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    fitting += 1;
                }
            }

            let start = logical.start + boundaries[start_index];
            let end = logical.start + boundaries[fitting];
            visual.push(start..end);
            start_index = fitting;
        }
    }
    visual
}

fn normalize_newlines(value: String) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn update_statistics(inner: &mut TextEditorInteractionInner) {
    inner.line_count = inner.value.bytes().filter(|byte| *byte == b'\n').count() + 1;
    inner.character_count = inner.value.chars().count();
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
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn previous_boundary(value: &str, cursor: usize) -> usize {
    value[..clamp_boundary(value, cursor)]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_boundary(value: &str, cursor: usize) -> usize {
    let cursor = clamp_boundary(value, cursor);
    value[cursor..]
        .chars()
        .next()
        .map(|character| cursor + character.len_utf8())
        .unwrap_or(cursor)
}

fn line_and_column(value: &str, cursor: usize, lines: &[Range<usize>]) -> (usize, usize) {
    let cursor = clamp_boundary(value, cursor);
    let line = lines
        .iter()
        .rposition(|range| cursor >= range.start && cursor <= range.end)
        .unwrap_or(lines.len().saturating_sub(1));
    let column = value[lines[line].start..cursor].chars().count();
    (line, column)
}

fn index_for_column(value: &str, range: &Range<usize>, column: usize) -> usize {
    value[range.clone()]
        .char_indices()
        .nth(column)
        .map(|(index, _)| range.start + index)
        .unwrap_or(range.end)
}

fn caret_state(interaction: &TextEditorInteractionState, now: Instant) -> (bool, Instant) {
    let mut inner = interaction.inner.borrow_mut();
    let origin = *inner.caret_blink_origin.get_or_insert(now);
    let elapsed = now.saturating_duration_since(origin).as_millis();
    let interval = CARET_BLINK_INTERVAL.as_millis();
    let visible = (elapsed / interval).is_multiple_of(2);
    (
        visible,
        now + Duration::from_millis((interval - elapsed % interval) as u64),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn line_navigation_uses_character_columns() {
        let value = "abc\n日本語\nx";
        let lines = line_ranges(value);
        assert_eq!(line_and_column(value, 2, &lines), (0, 2));
        assert_eq!(index_for_column(value, &lines[1], 2), 10);
        assert_eq!(index_for_column(value, &lines[2], 8), value.len());
    }

    #[test]
    fn visual_lines_wrap_at_character_boundaries_and_prefer_whitespace() {
        let mut measure = |value: String| value.chars().count() as f32 * 10.0;
        let ranges = visual_line_ranges("alpha beta", Some(60.0), &mut measure);
        let lines: Vec<&str> = ranges
            .iter()
            .map(|range| &"alpha beta"[range.clone()])
            .collect();
        assert_eq!(lines, ["alpha ", "beta"]);

        let mut measure = |value: String| value.chars().count() as f32 * 10.0;
        let ranges = visual_line_ranges("日本語文", Some(20.0), &mut measure);
        let lines: Vec<&str> = ranges
            .iter()
            .map(|range| &"日本語文"[range.clone()])
            .collect();
        assert_eq!(lines, ["日本", "語文"]);
    }

    #[test]
    fn cursor_at_soft_wrap_boundary_uses_the_continuation_line() {
        let value = "abcdef";
        let lines = vec![0..3, 3..6];
        assert_eq!(line_and_column(value, 3, &lines), (1, 0));
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

    #[test]
    fn adjacent_typing_is_one_undo_group() {
        let state = TextEditorInteractionState::new();
        state.set_value("");
        assert!(state.edit_with_kind(EditKind::Insert, |inner| {
            TextEditor::replace_selection(inner, "a")
        }));
        assert!(state.edit_with_kind(EditKind::Insert, |inner| {
            TextEditor::replace_selection(inner, "b")
        }));
        assert_eq!(state.value(), "ab");
        assert!(state.undo());
        assert_eq!(state.value(), "");
    }

    #[test]
    fn statistics_track_unicode_edits() {
        let state = TextEditorInteractionState::new();
        state.set_value("mochi\n餅");
        assert_eq!(state.statistics(), (2, 7));
        assert!(state.edit(|inner| TextEditor::replace_selection(inner, "!")));
        assert_eq!(state.statistics(), (2, 8));
    }

    #[test]
    fn find_wraps_and_selects_each_match() {
        let state = TextEditorInteractionState::new();
        state.set_value("one two one");
        assert_eq!(state.find_match("one", false, true), Some((1, 2)));
        assert_eq!(state.find_match("one", false, false), Some((2, 2)));
        assert_eq!(state.find_match("one", false, false), Some((1, 2)));
        assert_eq!(state.find_match("missing", false, true), None);
    }

    #[test]
    fn editor_scrollbar_reaches_document_end() {
        let viewport = Rect::new(10.0, 20.0, 200.0, 100.0);
        let document = Size::new(200.0, 300.0);
        let tokens = crate::theme::Theme::DEFAULT.scrollbar;
        let geometry =
            editor_scrollbar_geometry(viewport, document, Point::new(0.0, 200.0), tokens);
        let vertical = geometry.vertical.expect("vertical scrollbar");
        assert_eq!(vertical.maximum_offset, 200.0);
        assert_eq!(
            vertical.thumb.origin.y + vertical.thumb.size.height,
            vertical.track.origin.y + vertical.track.size.height,
        );
    }

    #[test]
    fn positive_wheel_delta_scrolls_document_forward() {
        let state = TextEditorInteractionState::new();
        state.set_value("line\n".repeat(100));
        let editor = TextEditor::with_interaction(state.clone());
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&Theme::DEFAULT, &Typography::DEFAULT, &mut measurer);
        let result = editor.handle_event(
            Rect::new(0.0, 0.0, 320.0, 200.0),
            &ViewEvent::Scroll {
                position: Point::new(40.0, 40.0),
                delta_x: 0.0,
                delta_y: 48.0,
            },
            &mut context,
        );

        assert_eq!(result, EventResult::Consumed);
        assert_eq!(state.inner.borrow().scroll_y, 48.0);
    }

    #[test]
    fn losing_window_focus_clears_the_caret_and_requests_repaint() {
        let state = TextEditorInteractionState::new();
        state.focus();
        let editor = TextEditor::with_interaction(state.clone());
        let bounds = Rect::new(0.0, 0.0, 320.0, 200.0);
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&Theme::DEFAULT, &Typography::DEFAULT, &mut measurer);

        let result = editor.handle_event(
            bounds,
            &ViewEvent::FocusChanged { focused: false },
            &mut context,
        );

        assert_eq!(result, EventResult::Ignored);
        assert!(!state.is_focused());
        assert_eq!(
            context.redraw_request(),
            crate::event::RedrawRequest::Region(bounds)
        );
    }
}
