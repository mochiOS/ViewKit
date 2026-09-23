//! Viewツリー内部で使用するイベント配送API

use crate::geometry::{Point, Rect};
use crate::accessibility::AccessibilityNode;
use crate::platform::{ButtonState, CursorIcon, Key, KeyModifiers, PlatformEvent, PointerButton};
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use crate::view::View;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenuItem {
    pub command_id: u32,
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub destructive: bool,
    pub separator: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuRequest {
    pub request_id: u64,
    pub position: Point,
    pub items: Vec<ContextMenuItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    PointerMoved {
        position: Point,
    },

    PointerPressed {
        position: Point,
        button: PointerButton,
    },

    PointerReleased {
        position: Point,
        button: PointerButton,
    },

    PointerLeft,

    KeyPressed {
        key: Key,
        modifiers: KeyModifiers,
    },

    Scroll {
        position: Point,
        delta_x: f32,
        delta_y: f32,
    },

    TextInput {
        text: String,
    },

    Backspace,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    Delete,

    SelectRight,
    SelectLeft,
    SelectHome,
    SelectEnd,
    SelectAll,

    PointerFocusRequested {
        position: Point,
    },

    KeyboardFocusRequested {
        bounds: Option<Rect>,
    },

    FocusChanged {
        focused: bool,
    },

    ContextMenuResult {
        request_id: u64,
        command_id: Option<u32>,
    },
}

impl ViewEvent {
    pub fn position(&self) -> Option<Point> {
        match self {
            Self::PointerMoved { position }
            | Self::PointerPressed { position, .. }
            | Self::PointerReleased { position, .. }
            | Self::Scroll { position, .. }
            | Self::PointerFocusRequested { position } => Some(*position),

            Self::PointerLeft
            | Self::KeyPressed { .. }
            | Self::TextInput { .. }
            | Self::FocusChanged { .. }
            | Self::KeyboardFocusRequested { .. }
            | Self::ContextMenuResult { .. }
            | Self::Backspace
            | Self::Delete
            | Self::ArrowLeft
            | Self::ArrowRight
            | Self::Home
            | Self::End
            | Self::SelectRight
            | Self::SelectLeft
            | Self::SelectHome
            | Self::SelectEnd
            | Self::SelectAll => None,
        }
    }

    pub fn is_inside(&self, bounds: Rect) -> bool {
        self.position()
            .map(|position| bounds.contains(position))
            .unwrap_or(true)
    }

    // TODO: ポインターキャプチャへ置き換える
    pub fn requires_broadcast(&self) -> bool {
        matches!(
            self,
            Self::PointerMoved { .. }
                | Self::PointerReleased { .. }
                | Self::PointerFocusRequested { .. }
                | Self::KeyboardFocusRequested { .. }
                | Self::PointerLeft
                | Self::KeyPressed { .. }
                | Self::TextInput { .. }
                | Self::Backspace
                | Self::Delete
                | Self::Home
                | Self::End
                | Self::ArrowLeft
                | Self::ArrowRight
                | Self::FocusChanged { .. }
                | Self::ContextMenuResult { .. }
                | Self::SelectLeft
                | Self::SelectRight
                | Self::SelectHome
                | Self::SelectEnd
                | Self::SelectAll
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventResult {
    #[default]
    Ignored,

    Consumed,
}

impl EventResult {
    pub fn is_consumed(self) -> bool {
        self == Self::Consumed
    }

    pub fn merge(self, other: Self) -> Self {
        if self.is_consumed() || other.is_consumed() {
            Self::Consumed
        } else {
            Self::Ignored
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RedrawRequest {
    #[default]
    None,

    Full,

    Region(Rect),
}

impl RedrawRequest {
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Full, _) | (_, Self::Full) => Self::Full,

            (Self::None, request) | (request, Self::None) => request,

            (Self::Region(first), Self::Region(second)) => Self::Region(first.union(second)),
        }
    }

    pub fn is_requested(self) -> bool {
        !matches!(self, Self::None)
    }
}

pub struct EventContext<'a> {
    pub(crate) theme: &'a Theme,
    pub(crate) typography: &'a Typography,
    pub(crate) text_measurer: &'a mut TextMeasurer,

    redraw_request: RedrawRequest,
    cursor_icon: Option<CursorIcon>,
    context_menu_request: Option<ContextMenuRequest>,
    keyboard_focus_request: Option<Option<Rect>>,
}

impl<'a> EventContext<'a> {
    pub fn new(
        theme: &'a Theme,
        typography: &'a Typography,
        text_measurer: &'a mut TextMeasurer,
    ) -> Self {
        Self {
            theme,
            typography,
            text_measurer,
            redraw_request: RedrawRequest::None,
            cursor_icon: None,
            context_menu_request: None,
            keyboard_focus_request: None,
        }
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    pub fn typography(&self) -> &Typography {
        self.typography
    }

    pub fn request_redraw(&mut self) {
        self.redraw_request = RedrawRequest::Full;
    }

    pub fn request_redraw_in(&mut self, bounds: Rect) {
        if bounds.is_empty() {
            return;
        }

        self.redraw_request = self.redraw_request.merge(RedrawRequest::Region(bounds));
    }

    pub fn redraw_request(&self) -> RedrawRequest {
        self.redraw_request
    }

    pub fn set_cursor(&mut self, cursor_icon: CursorIcon) {
        self.cursor_icon = Some(cursor_icon);
    }

    pub fn cursor_icon(&self) -> Option<CursorIcon> {
        self.cursor_icon
    }

    pub fn show_context_menu(&mut self, request: ContextMenuRequest) {
        self.context_menu_request = Some(request);
    }

    pub fn take_context_menu_request(&mut self) -> Option<ContextMenuRequest> {
        self.context_menu_request.take()
    }

    /// Requests keyboard focus for a focusable accessibility node with these
    /// exact bounds. The dispatcher validates the target against the current
    /// focus order before applying it.
    pub fn request_keyboard_focus(&mut self, bounds: Rect) {
        self.keyboard_focus_request = Some(Some(bounds));
    }

    pub fn clear_keyboard_focus(&mut self) {
        self.keyboard_focus_request = Some(None);
    }

    fn take_keyboard_focus_request(&mut self) -> Option<Option<Rect>> {
        self.keyboard_focus_request.take()
    }
}

#[derive(Clone, Debug, Default)]
pub struct EventDispatcher {
    pointer_position: Option<Point>,
    focus_order: Vec<Rect>,
    focused_bounds: Option<Rect>,
    focus_scopes: Vec<FocusScopeState>,
    pending_focus_request: Option<Option<Rect>>,
}

#[derive(Clone, Debug)]
struct FocusScopeState {
    bounds: Rect,
    restore_focus: Option<Rect>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pointer_position(&self) -> Option<Point> {
        self.pointer_position
    }

    pub fn set_accessibility_nodes(&mut self, nodes: &[AccessibilityNode]) {
        let scope_entries: Vec<(usize, Rect)> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.focus_scope)
            .map(|(index, node)| (index, node.bounds))
            .collect();
        let common_scope_count = self
            .focus_scopes
            .iter()
            .zip(scope_entries.iter())
            .take_while(|(active, (_, bounds))| active.bounds == *bounds)
            .count();

        while self.focus_scopes.len() > common_scope_count {
            if let Some(scope) = self.focus_scopes.pop() {
                self.focused_bounds = scope.restore_focus;
                self.pending_focus_request = Some(self.focused_bounds);
            }
        }

        for (scope_position, (scope_index, bounds)) in scope_entries
            .iter()
            .enumerate()
            .skip(common_scope_count)
        {
            let next_scope_index = scope_entries
                .get(scope_position + 1)
                .map(|(index, _)| *index)
                .unwrap_or(nodes.len());
            let target = nodes[scope_index + 1..next_scope_index]
                .iter()
                .find(|node| node.focusable && node.enabled)
                .map(|node| node.bounds)
                .or_else(|| {
                    nodes[scope_index + 1..]
                        .iter()
                        .find(|node| node.focusable && node.enabled)
                        .map(|node| node.bounds)
                });
            self.focus_scopes.push(FocusScopeState {
                bounds: *bounds,
                restore_focus: self.focused_bounds,
            });
            self.focused_bounds = target;
            self.pending_focus_request = Some(target);
        }

        self.focus_order.clear();
        let focus_start = scope_entries
            .last()
            .map(|(index, _)| index + 1)
            .unwrap_or(0);
        self.focus_order.extend(
            nodes[focus_start..]
                .iter()
                .filter(|node| node.focusable && node.enabled)
                .map(|node| node.bounds),
        );

        if self
            .focused_bounds
            .is_some_and(|focused| !self.focus_order.contains(&focused))
        {
            self.focused_bounds = self.focus_order.first().copied();
            self.pending_focus_request = Some(self.focused_bounds);
        }
    }

    fn next_focus(&self, backwards: bool) -> Option<Rect> {
        let count = self.focus_order.len();
        if count == 0 {
            return None;
        }

        let current = self
            .focused_bounds
            .and_then(|focused| self.focus_order.iter().position(|bounds| *bounds == focused));
        let index = match (current, backwards) {
            (Some(0), true) | (None, true) => count - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % count,
            (None, false) => 0,
        };
        self.focus_order.get(index).copied()
    }

    pub fn dispatch(
        &mut self,
        root: &dyn View,
        bounds: Rect,
        event: &PlatformEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let mut result = EventResult::Ignored;

        if let Some(target) = self.pending_focus_request.take() {
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::KeyboardFocusRequested { bounds: target },
                context,
            ));
        }

        if let PlatformEvent::KeyPressed {
            key: Key::Tab,
            modifiers,
        } = event
        {
            // Give the focused control the first opportunity to consume Tab.
            // Multi-line editors use it as document input, while controls that
            // ignore it retain the normal keyboard focus traversal behavior.
            let tab_result = root.handle_event(
                bounds,
                &ViewEvent::KeyPressed {
                    key: Key::Tab,
                    modifiers: *modifiers,
                },
                context,
            );
            result = result.merge(tab_result);
            if tab_result.is_consumed() {
                return result;
            }

            let target = self.next_focus(modifiers.shift());
            self.focused_bounds = target;
            return result.merge(root.handle_event(
                bounds,
                &ViewEvent::KeyboardFocusRequested { bounds: target },
                context,
            ));
        }

        let is_primary_press = matches!(
            event,
            PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            }
        );

        if is_primary_press && let Some(position) = self.pointer_position {
            let target = self
                .focus_order
                .iter()
                .copied()
                .find(|bounds| bounds.contains(position));
            self.focused_bounds = target;
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::KeyboardFocusRequested { bounds: target },
                context,
            ));
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::PointerFocusRequested { position },
                context,
            ));
        }

        let Some(view_event) = self.convert_event(event) else {
            return result;
        };

        result = result.merge(root.handle_event(bounds, &view_event, context));

        if let Some(requested) = context.take_keyboard_focus_request() {
            let target = requested.filter(|bounds| self.focus_order.contains(bounds));
            self.focused_bounds = target;
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::KeyboardFocusRequested { bounds: target },
                context,
            ));
        }

        result
    }

    fn convert_event(&mut self, event: &PlatformEvent) -> Option<ViewEvent> {
        match event {
            PlatformEvent::PointerMoved { x, y } => {
                let position = Point::new(*x, *y);

                self.pointer_position = Some(position);

                Some(ViewEvent::PointerMoved { position })
            }

            PlatformEvent::PointerButton { button, state } => {
                let position = self.pointer_position?;

                match state {
                    ButtonState::Pressed => Some(ViewEvent::PointerPressed {
                        position,
                        button: *button,
                    }),

                    ButtonState::Released => Some(ViewEvent::PointerReleased {
                        position,
                        button: *button,
                    }),
                }
            }

            PlatformEvent::PointerLeft => {
                self.pointer_position = None;

                Some(ViewEvent::PointerLeft)
            }

            PlatformEvent::KeyPressed { key, modifiers } => Some(ViewEvent::KeyPressed {
                key: *key,
                modifiers: *modifiers,
            }),

            PlatformEvent::Scroll { delta_x, delta_y } => {
                let position = self.pointer_position?;

                Some(ViewEvent::Scroll {
                    position,
                    delta_x: *delta_x,
                    delta_y: *delta_y,
                })
            }

            PlatformEvent::Focused(focused) => {
                if !focused {
                    self.pointer_position = None;
                    self.focused_bounds = None;
                }

                Some(ViewEvent::FocusChanged { focused: *focused })
            }

            PlatformEvent::ContextMenuResult {
                request_id,
                command_id,
            } => Some(ViewEvent::ContextMenuResult {
                request_id: *request_id,
                command_id: *command_id,
            }),

            PlatformEvent::TextInput { text } => Some(ViewEvent::TextInput { text: text.clone() }),
            PlatformEvent::Backspace => Some(ViewEvent::Backspace),
            PlatformEvent::Delete => Some(ViewEvent::Delete),
            PlatformEvent::ArrowLeft => Some(ViewEvent::ArrowLeft),
            PlatformEvent::ArrowRight => Some(ViewEvent::ArrowRight),
            PlatformEvent::Home => Some(ViewEvent::Home),
            PlatformEvent::End => Some(ViewEvent::End),

            PlatformEvent::SelectLeft => Some(ViewEvent::SelectLeft),
            PlatformEvent::SelectRight => Some(ViewEvent::SelectRight),
            PlatformEvent::SelectHome => Some(ViewEvent::SelectHome),
            PlatformEvent::SelectEnd => Some(ViewEvent::SelectEnd),
            PlatformEvent::SelectAll => Some(ViewEvent::SelectAll),

            PlatformEvent::Resumed { .. }
            | PlatformEvent::Resized { .. }
            | PlatformEvent::ScaleFactorChanged { .. }
            | PlatformEvent::RedrawRequested
            | PlatformEvent::CloseRequested => None,
        }
    }
}
