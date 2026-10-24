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
use zeroize::{Zeroize, Zeroizing};

use super::input_method::{
    CandidatePanelLayout, InputMethodState, paint_candidates, paint_input_mode_indicator,
};

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::animation::AppearanceTransition;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const SECURE_GLYPH: char = '\u{2022}';

#[derive(Clone, Default, PartialEq)]
struct TextFieldInteractionInner {
    hovered: bool,
    focused: bool,
    focus_visible: bool,
    appearance: AppearanceTransition,
    enabled: bool,
    value: Zeroizing<String>,
    sensitive: bool,
    cursor: usize,
    scroll_offset_x: f32,
    selection_anchor: Option<usize>,
    selecting: bool,
    input_method: InputMethodState,
    candidate_panel: Option<CandidatePanelLayout>,
    hovered_candidate: Option<usize>,

    value_initialized: bool,
    caret_blink_origin: Option<Instant>,
}
impl core::fmt::Debug for TextFieldInteractionInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TextFieldInteractionInner")
            .field("focused", &self.focused)
            .field("value", &"[REDACTED]")
            .finish_non_exhaustive()
    }
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
        if !focused {
            inner.focus_visible = false;
        }

        changed
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();

        inner.hovered = false;
        inner.focused = false;
        inner.focus_visible = false;
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
        self.inner.borrow().value.to_string()
    }
    /// 値の所有権を移し、入力欄を空にします。秘密情報は呼出側で消去してください。
    pub fn take_value(&self) -> String {
        let mut inner = self.inner.borrow_mut();
        let value = std::mem::take(&mut *inner.value);
        inner.cursor = 0;
        inner.scroll_offset_x = 0.0;
        inner.selection_anchor = None;
        inner.selecting = false;
        value
    }

    /// 選択範囲をUTF-8のバイト位置で返します。未選択の場合はキャレット位置です。
    pub fn selection_range(&self) -> Range<usize> {
        let inner = self.inner.borrow();
        selection_range(&inner).unwrap_or(inner.cursor..inner.cursor)
    }

    /// UTF-8境界へ切り下げて選択範囲を設定します。
    pub fn set_selection(&self, range: Range<usize>) {
        let mut inner = self.inner.borrow_mut();
        let boundary = |mut index: usize| {
            index = index.min(inner.value.len());
            while !inner.value.is_char_boundary(index) {
                index -= 1;
            }
            index
        };
        let start = boundary(range.start);
        let end = boundary(range.end);
        inner.cursor = end;
        inner.selection_anchor = (start != end).then_some(start);
        inner.selecting = false;
    }

    pub fn set_value(&self, value: impl Into<String>) {
        let value = value.into();
        let mut inner = self.inner.borrow_mut();

        inner.cursor = value.len();
        inner.scroll_offset_x = 0.0;
        inner.value = Zeroizing::new(value);
        inner.value_initialized = true;
        inner.selection_anchor = None;
        inner.selecting = false;
    }

    /// 値が保持していた領域をNULで上書きしてから空にします。
    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.value.zeroize();
        inner.cursor = 0;
        inner.scroll_offset_x = 0.0;
        inner.selection_anchor = None;
        inner.selecting = false;
    }

    fn initialize_value(&self, value: String) {
        let value = Zeroizing::new(value);
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

        replace_value_range(&mut inner, cursor..cursor + character_length, "");

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
        let mut filtered = Zeroizing::new(String::with_capacity(text.len()));
        filtered.extend(text.chars().filter(|character| !character.is_control()));

        if filtered.is_empty() {
            return false;
        }

        let mut inner = self.inner.borrow_mut();

        delete_selection(&mut inner);
        let cursor = inner.cursor.min(inner.value.len());
        replace_value_range(&mut inner, cursor..cursor, filtered.as_str());
        inner.cursor = cursor + filtered.len();
        inner.selection_anchor = None;
        inner.selecting = false;

        true
    }

    fn input_method_text(&self, text: &str) -> bool {
        let mut inner = self.inner.borrow_mut();
        let selection = selection_range(&inner);
        let mut input_method = std::mem::take(&mut inner.input_method);
        let mut cursor = inner.cursor;
        let handled = input_method.handle_text(&mut inner.value, &mut cursor, selection, text);
        inner.cursor = cursor;
        if handled {
            inner.selection_anchor = None;
            inner.selecting = false;
        }
        inner.input_method = input_method;
        handled
    }

    fn input_method_action(&self, action: InputMethodAction) -> bool {
        let mut inner = self.inner.borrow_mut();
        let mut input_method = std::mem::take(&mut inner.input_method);
        let mut cursor = inner.cursor;
        let handled = match action {
            InputMethodAction::Next(delta) => {
                input_method.select_next(&mut inner.value, &mut cursor, delta)
            }
            InputMethodAction::PreviousFromSpace => {
                input_method.select_previous_from_space(&mut inner.value, &mut cursor)
            }
            InputMethodAction::Page(delta) => {
                input_method.select_page(&mut inner.value, &mut cursor, delta)
            }
            InputMethodAction::Choose(index) => {
                input_method.choose(&mut inner.value, &mut cursor, index)
            }
            InputMethodAction::Commit => {
                input_method.commit_before_text(&mut inner.value, &mut cursor, '\n')
            }
            InputMethodAction::CommitForNavigation => {
                input_method.commit(&mut inner.value, &mut cursor)
            }
            InputMethodAction::Cancel => input_method.cancel(&mut inner.value, &mut cursor),
            InputMethodAction::Backspace => input_method.backspace(&mut inner.value, &mut cursor),
            InputMethodAction::Toggle => {
                input_method.commit(&mut inner.value, &mut cursor);
                input_method.toggle();
                true
            }
            InputMethodAction::Synchronize => {
                input_method.commit(&mut inner.value, &mut cursor);
                input_method.synchronize();
                true
            }
        };
        inner.cursor = cursor;
        inner.input_method = input_method;
        handled
    }

    fn commit_input_method_on_blur(&self) {
        let mut inner = self.inner.borrow_mut();
        let mut input_method = std::mem::take(&mut inner.input_method);
        let mut cursor = inner.cursor;
        input_method.commit_on_blur(&mut inner.value, &mut cursor);
        inner.cursor = cursor;
        inner.input_method = input_method;
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

        replace_value_range(&mut inner, previous..cursor, "");
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
            Self::Small => theme.controls.small_height,
            Self::Medium => theme.controls.height,
            Self::Large => theme.controls.large_height,
        }
    }

    fn horizontal_padding(self, theme: &crate::theme::Theme) -> f32 {
        theme.text_field.horizontal_padding
    }

    const fn text_role(self) -> TextRole {
        match self {
            Self::Small => TextRole::Caption,
            Self::Medium => TextRole::Label,
            Self::Large => TextRole::Body,
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
    radius: Option<CornerRadius>,
    leading_symbol: Option<SymbolName>,

    enabled: bool,
    invalid: bool,
    secure: bool,
    bordered: bool,
    on_submit: Option<RefCell<Box<dyn FnMut()>>>,
}

#[derive(Clone, Copy)]
enum InputMethodAction {
    Next(isize),
    PreviousFromSpace,
    Page(isize),
    Choose(usize),
    Commit,
    CommitForNavigation,
    Cancel,
    Backspace,
    Toggle,
    Synchronize,
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
            radius: None,
            leading_symbol: None,

            enabled: true,
            invalid: false,
            secure: false,
            bordered: true,
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
            radius: None,
            leading_symbol: None,

            enabled: true,
            invalid: false,
            secure: false,
            bordered: true,
            on_submit: None,
        }
    }

    pub fn value(self, value: impl Into<String>) -> Self {
        self.interaction.initialize_value(value.into());

        self
    }

    /// 背景と輪郭の描画を切り替えます。入力・選択・IME・フォーカス操作は維持します。
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
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
        self.radius = Some(radius);
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
        self.radius = Some(CornerRadius::Full);
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
        if secure {
            let mut inner = self.interaction.inner.borrow_mut();
            inner.sensitive = true;
            inner.input_method = InputMethodState::default();
        }
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
        let value = self.interaction.inner.borrow().value.clone();

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
        } else if interaction.focused && interaction.focus_visible {
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
        let (appearance, next) = self.interaction.inner.borrow_mut().appearance.sample(
            crate::theme::ControlAppearance {
                background: appearance.background,
                border: appearance.border,
                foreground: appearance.foreground,
            },
            context.theme.motion.fast,
            Instant::now(),
            !self.enabled || self.invalid,
        );
        if let Some(next) = next {
            context.request_redraw_in_at(bounds.expanded(16.0), next);
        }

        let (value, cursor, focused, stored_scroll_offset_x, selection, marked, converting) = {
            let inner = self.interaction.inner.borrow();

            (
                inner.value.clone(),
                inner.cursor.min(inner.value.len()),
                inner.focused && inner.enabled,
                inner.scroll_offset_x,
                selection_range(&inner),
                inner.input_method.marked_range(),
                inner.input_method.is_converting(),
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
            accessibility.value = Some(value.to_string());
        }
        accessibility.enabled = self.enabled;
        accessibility.focusable = true;
        accessibility.focused = focused;
        accessibility.invalid = self.invalid;
        context.record_accessibility(accessibility);

        let radius = self.radius.unwrap_or(context.theme.text_field.radius);
        if focused && self.interaction.inner.borrow().focus_visible {
            let ring_width = context.theme.text_field.focus_ring_width;

            let radius =
                radius.resolve(&context.theme.radius, bounds.size.width, bounds.size.height);

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

        if self.bordered {
            Rectangle::new()
                .color(RectangleColor::Custom(appearance.background))
                .radius(radius)
                .shadow(ShadowStyle::None)
                .border(BorderStyle::custom(
                    appearance.border,
                    context.theme.text_field.stroke_width,
                ))
                .paint(bounds, context);
        }

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

                if converting && let Some(range) = marked.as_ref() {
                    let start_width = Text::styled(
                        self.display_prefix(&value, range.start),
                        self.size.text_role(),
                    )
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;
                    let end_width = Text::styled(
                        self.display_prefix(&value, range.end),
                        self.size.text_role(),
                    )
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;
                    let left = (text_bounds.origin.x + start_width - scroll_offset_x)
                        .max(text_bounds.origin.x);
                    let right = (text_bounds.origin.x + end_width - scroll_offset_x)
                        .min(text_bounds.origin.x + text_bounds.size.width);
                    if right > left {
                        context
                            .display_list
                            .push(DrawCommand::PushClip { rect: text_bounds });
                        context.display_list.push(DrawCommand::FillRoundedRect {
                            rect: Rect::new(
                                left,
                                text_bounds.origin.y,
                                right - left,
                                text_bounds.size.height,
                            ),
                            radius: context.theme.text_field.selection_radius,
                            color: context.theme.colors.accent_soft,
                        });
                        context.display_list.push(DrawCommand::PopClip);
                    }
                }

                Text::styled(display_text, self.size.text_role())
                    .accessibility_hidden(true)
                    .color(appearance.foreground)
                    .paint(content_bounds, context);

                if let Some(range) = marked.as_ref() {
                    let start_width = Text::styled(
                        self.display_prefix(&value, range.start),
                        self.size.text_role(),
                    )
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;
                    let end_width = Text::styled(
                        self.display_prefix(&value, range.end),
                        self.size.text_role(),
                    )
                    .measure_unbounded_with_typography(context.text_measurer, context.typography)
                    .width;
                    let left = (text_bounds.origin.x + start_width - scroll_offset_x)
                        .max(text_bounds.origin.x);
                    let right = (text_bounds.origin.x + end_width - scroll_offset_x)
                        .min(text_bounds.origin.x + text_bounds.size.width);
                    if right > left {
                        context
                            .display_list
                            .push(DrawCommand::PushClip { rect: text_bounds });
                        context.display_list.push(DrawCommand::FillRect {
                            rect: Rect::new(
                                left,
                                text_bounds.origin.y + text_bounds.size.height
                                    - if converting { 2.0 } else { 1.0 },
                                right - left,
                                if converting { 2.0 } else { 1.0 },
                            ),
                            color: context.theme.colors.accent,
                        });
                        context.display_list.push(DrawCommand::PopClip);
                    }
                }
            }
        }

        if focused && !self.secure {
            let anchor_x = (text_bounds.origin.x + prefix_width - scroll_offset_x).clamp(
                text_bounds.origin.x,
                text_bounds.origin.x + text_bounds.size.width,
            );
            let now = Instant::now();
            let anchor = Rect::new(anchor_x, text_bounds.origin.y, 1.0, text_bounds.size.height);
            let (candidate_panel, mode_indicator) = {
                let inner = self.interaction.inner.borrow();
                let (candidates, selected) = inner.input_method.candidates();
                (
                    paint_candidates(
                        anchor,
                        candidates,
                        selected,
                        inner.hovered_candidate,
                        context,
                    ),
                    inner.input_method.mode_indicator(now),
                )
            };
            self.interaction.inner.borrow_mut().candidate_panel = candidate_panel;
            if let Some((japanese, expires_at)) = mode_indicator {
                paint_input_mode_indicator(anchor, japanese, context);
                context.request_redraw_in_at(bounds.expanded(16.0), expires_at);
            }
        } else {
            let mut inner = self.interaction.inner.borrow_mut();
            inner.candidate_panel = None;
            inner.hovered_candidate = None;
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
        let radius = self
            .radius
            .unwrap_or(context.theme.text_field.radius)
            .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);
        let enabled_changed = self.interaction.set_enabled(self.enabled);

        if enabled_changed {
            context.request_redraw_in(bounds.expanded(16.0));
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            ViewEvent::KeyPressed {
                key: Key::CapsLock, ..
            } if self.interaction.is_focused() && !self.secure => {
                self.interaction
                    .input_method_action(InputMethodAction::Synchronize);
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let should_focus = target.is_some_and(|target| target == bounds);
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != should_focus
                    || inner.focus_visible != (should_focus && context.focus_visible());
                inner.focused = should_focus && inner.enabled;
                inner.focus_visible = should_focus && context.focus_visible();
                if !should_focus {
                    let mut input_method = std::mem::take(&mut inner.input_method);
                    let mut cursor = inner.cursor;
                    input_method.commit_on_blur(&mut inner.value, &mut cursor);
                    inner.cursor = cursor;
                    inner.input_method = input_method;
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
                let hovered = bounds.contains_rounded(radius, *position);
                let selecting = {
                    let mut inner = self.interaction.inner.borrow_mut();
                    let candidate_hover = inner
                        .candidate_panel
                        .as_ref()
                        .and_then(|panel| panel.candidate_at(*position));
                    let hover_changed = inner.hovered != hovered;
                    let candidate_hover_changed = inner.hovered_candidate != candidate_hover;
                    inner.hovered = hovered;
                    inner.hovered_candidate = candidate_hover;

                    if hover_changed || candidate_hover_changed {
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
                let candidate = self
                    .interaction
                    .inner
                    .borrow()
                    .candidate_panel
                    .as_ref()
                    .and_then(|panel| panel.candidate_at(*position));
                if let Some(candidate) = candidate
                    && self.interaction.is_focused()
                    && !self.secure
                    && self
                        .interaction
                        .input_method_action(InputMethodAction::Choose(candidate))
                {
                    self.interaction.inner.borrow_mut().hovered_candidate = None;
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.intercept_pointer_press();
                    context.request_redraw();
                    return EventResult::Consumed;
                }
                let should_focus = bounds.contains_rounded(radius, *position);
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
                if !bounds.contains_rounded(radius, *position) {
                    if self.interaction.set_focused(false) {
                        context.request_redraw_in(bounds.expanded(16.0));
                    }
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
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

                let changed = inner.hovered || inner.hovered_candidate.is_some();

                inner.hovered = false;
                inner.hovered_candidate = None;

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            ViewEvent::FocusChanged { focused: false } => {
                self.interaction.commit_input_method_on_blur();
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.hovered || inner.focused;

                inner.hovered = false;
                inner.focused = false;
                inner.candidate_panel = None;
                inner.hovered_candidate = None;

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

                if (!self.secure && self.interaction.input_method_text(text))
                    || self.interaction.insert_text(text)
                {
                    self.synchronize_binding();
                    self.interaction.reset_caret_blink();
                    context.request_redraw_in(bounds.expanded(220.0));
                }

                EventResult::Consumed
            }

            ViewEvent::KeyPressed {
                key: Key::Space,
                modifiers,
            } if self.interaction.is_focused() && modifiers.control() && !self.secure => {
                self.interaction
                    .input_method_action(InputMethodAction::Toggle);
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'c' || *character == 'C') =>
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

            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'x' || *character == 'X') =>
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

            ViewEvent::KeyPressed {
                key: Key::Character(character),
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shortcut()
                && (*character == 'v' || *character == 'V') =>
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

            ViewEvent::KeyPressed {
                key: Key::Space,
                modifiers,
            } if self.interaction.is_focused()
                && modifiers.shift()
                && !modifiers.shortcut()
                && !self.secure =>
            {
                if self
                    .interaction
                    .input_method_action(InputMethodAction::PreviousFromSpace)
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::KeyPressed {
                key: Key::PageDown, ..
            } if self.interaction.is_focused() && !self.secure => {
                if self
                    .interaction
                    .input_method_action(InputMethodAction::Page(1))
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::KeyPressed {
                key: Key::PageUp, ..
            } if self.interaction.is_focused() && !self.secure => {
                if self
                    .interaction
                    .input_method_action(InputMethodAction::Page(-1))
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::KeyPressed {
                key: Key::ArrowDown,
                ..
            } if self.interaction.is_focused() && !self.secure => {
                if self
                    .interaction
                    .input_method_action(InputMethodAction::Next(1))
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::KeyPressed {
                key: Key::ArrowUp, ..
            } if self.interaction.is_focused() && !self.secure => {
                if self
                    .interaction
                    .input_method_action(InputMethodAction::Next(-1))
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::KeyPressed {
                key: Key::Escape, ..
            } if self.interaction.is_focused() && !self.secure => {
                if self
                    .interaction
                    .input_method_action(InputMethodAction::Cancel)
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::KeyPressed {
                key: Key::Enter, ..
            } => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }
                if !self.secure
                    && self
                        .interaction
                        .input_method_action(InputMethodAction::Commit)
                {
                    self.synchronize_binding();
                    context.request_redraw_in(bounds.expanded(220.0));
                } else if let Some(callback) = self.on_submit.as_ref() {
                    (callback.borrow_mut())();
                }
                EventResult::Consumed
            }

            ViewEvent::Backspace => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                if (!self.secure
                    && self
                        .interaction
                        .input_method_action(InputMethodAction::Backspace))
                    || self.interaction.delete_backward()
                {
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

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.move_cursor_left();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::ArrowRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.move_cursor_right();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::Home => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.move_cursor_home();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::End => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.move_cursor_end();
                self.interaction.reset_caret_blink();
                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectLeft => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.extend_selection_left();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectRight => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.extend_selection_right();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectHome => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
                self.interaction.extend_selection_home();

                self.interaction.reset_caret_blink();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::SelectEnd => {
                if !self.interaction.is_focused() {
                    return EventResult::Ignored;
                }

                self.interaction
                    .input_method_action(InputMethodAction::CommitForNavigation);
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

    replace_value_range(inner, range.clone(), "");

    inner.cursor = range.start;
    inner.selection_anchor = None;
    inner.selecting = false;

    true
}

fn replace_value_range(inner: &mut TextFieldInteractionInner, range: Range<usize>, text: &str) {
    if !inner.sensitive {
        inner.value.replace_range(range, text);
        return;
    }
    // String insertion may realloc, while deletion leaves copies in spare
    // capacity. Build separately so every superseded secret buffer is erased.
    let length = inner.value.len() - range.len() + text.len();
    let mut next = Zeroizing::new(String::with_capacity(length));
    next.push_str(&inner.value[..range.start]);
    next.push_str(text);
    next.push_str(&inner.value[range.end..]);
    inner.value = next;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    struct AuditAllocator;
    static WATCHED: AtomicUsize = AtomicUsize::new(0);
    static WATCHED_LEN: AtomicUsize = AtomicUsize::new(0);
    static RELEASED: AtomicBool = AtomicBool::new(false);
    static UNWIPED: AtomicBool = AtomicBool::new(false);
    fn inspect_release(ptr: *mut u8, layout: Layout) {
        if WATCHED
            .compare_exchange(ptr as usize, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let len = WATCHED_LEN.load(Ordering::SeqCst).min(layout.size());
            // Only initialized bytes, while this allocation is still live.
            let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
            UNWIPED.store(bytes.iter().any(|&b| b != 0), Ordering::SeqCst);
            RELEASED.store(true, Ordering::SeqCst);
        }
    }
    unsafe impl GlobalAlloc for AuditAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            inspect_release(ptr, layout);
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            inspect_release(ptr, layout);
            unsafe { System.realloc(ptr, layout, size) }
        }
    }
    #[global_allocator]
    static ALLOCATOR: AuditAllocator = AuditAllocator;
    fn watch_value(state: &TextFieldInteractionState) {
        let inner = state.inner.borrow();
        RELEASED.store(false, Ordering::SeqCst);
        UNWIPED.store(false, Ordering::SeqCst);
        WATCHED_LEN.store(inner.value.len(), Ordering::SeqCst);
        WATCHED.store(inner.value.as_ptr() as usize, Ordering::SeqCst);
    }
    fn assert_erased() {
        assert!(RELEASED.load(Ordering::SeqCst));
        assert!(!UNWIPED.load(Ordering::SeqCst));
    }
    #[test]
    fn secure_editing_and_drop_erase_released_buffers() {
        let state = TextFieldInteractionState::new();
        let field = TextField::with_interaction(state.clone()).secure(true);
        state.set_value("秘密 password");
        assert!(!format!("{:?}", state.inner.borrow()).contains("password"));
        watch_value(&state);
        state.insert_text(" and more");
        assert_erased();
        state.set_selection(0..6);
        watch_value(&state);
        state.delete_backward();
        assert_erased();
        assert_eq!(state.value(), " password and more");
        watch_value(&state);
        let extracted = Zeroizing::new(state.take_value());
        assert!(state.value().is_empty());
        assert!(!RELEASED.load(Ordering::SeqCst));
        drop(extracted);
        assert_erased();
        state.set_value("erase on destruction");
        watch_value(&state);
        drop(field);
        drop(state);
        assert_erased();
    }

    #[test]
    fn programmatic_selection_is_utf8_safe_and_plain_fields_keep_input() {
        let state = TextFieldInteractionState::new();
        state.set_value("12あ34");
        state.set_selection(3..5);
        assert_eq!(state.selection_range(), 2..5);
        state.set_selection(usize::MAX..usize::MAX);
        assert_eq!(state.selection_range(), 7..7);
        state.set_focused(true);
        let field = TextField::with_interaction(state.clone()).bordered(false);
        let bounds = Rect::new(0.0, 0.0, 240.0, 32.0);
        let theme = crate::theme::Theme::current();
        let mut measurer = crate::typography::TextMeasurer::new();
        let mut list = crate::draw_command::DisplayList::new();
        field.paint(
            bounds,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut measurer),
        );
        assert!(!list.commands().iter().any(
            |cmd| matches!(cmd, DrawCommand::FillRoundedRect { rect, .. } if *rect == bounds)
        ));
        field.handle_event(
            bounds,
            &ViewEvent::TextInput { text: "5".into() },
            &mut EventContext::new(&theme, &theme.typography, &mut measurer),
        );
        assert_eq!(state.value(), "12あ345");
        assert!(TextField::new(crate::state::State::new(String::new()).binding()).bordered);
    }

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
