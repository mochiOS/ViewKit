//! Viewツリー内部で使用するイベント配送API

use crate::accessibility::AccessibilityNode;
use crate::command::{CommandId, CommandStatus};
use crate::geometry::{Point, Rect};
use crate::platform::{ButtonState, CursorIcon, Key, KeyModifiers, PlatformEvent, PointerButton};
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use crate::view::View;
use std::path::PathBuf;

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

impl ContextMenuRequest {
    /// Creates a native menu request with an identity shared with ViewKit's
    /// built-in menus. Use the returned `request_id` to match its result;
    /// application-local counters can collide with menus opened by controls.
    pub fn new(position: Point, items: Vec<ContextMenuItem>) -> Self {
        Self {
            request_id: Self::next_id(),
            position,
            items,
        }
    }

    pub(crate) fn next_id() -> u64 {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .max(1)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    /// Runtime lifecycle notification to old view trees after a rebuild.
    /// Identified components absent from the new tree release interaction.
    ReconcileComponents {
        retained: Vec<crate::accessibility::AccessibilityIdentity>,
    },
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

    KeyReleased {
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

    FileDragEntered {
        path: PathBuf,
        position: Option<Point>,
    },

    FileDragExited,

    FileDropped {
        path: PathBuf,
        position: Option<Point>,
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

    FocusVisibilityChanged {
        visible: bool,
    },

    FocusChanged {
        focused: bool,
    },

    /// The platform refused a temporary keyboard capture request.
    KeyboardCaptureDenied,

    ContextMenuResult {
        request_id: u64,
        command_id: Option<u32>,
    },

    /// A semantic action routed from the focused view toward its ancestors.
    /// Current modifier state for pointer and keyboard interactions.
    ModifiersChanged {
        modifiers: KeyModifiers,
    },

    Command {
        command: CommandId,
        target: Option<Point>,
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

            Self::Command { target, .. } => *target,

            Self::FileDragEntered { position, .. } | Self::FileDropped { position, .. } => {
                *position
            }

            Self::ReconcileComponents { .. }
            | Self::PointerLeft
            | Self::ModifiersChanged { .. }
            | Self::KeyPressed { .. }
            | Self::KeyReleased { .. }
            | Self::TextInput { .. }
            | Self::FileDragExited
            | Self::FocusChanged { .. }
            | Self::KeyboardCaptureDenied
            | Self::KeyboardFocusRequested { .. }
            | Self::FocusVisibilityChanged { .. }
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
            Self::ReconcileComponents { .. }
                | Self::PointerMoved { .. }
                | Self::PointerReleased { .. }
                | Self::PointerFocusRequested { .. }
                | Self::KeyboardFocusRequested { .. }
                | Self::FocusVisibilityChanged { .. }
                | Self::PointerLeft
                | Self::ModifiersChanged { .. }
                | Self::KeyPressed { .. }
                | Self::TextInput { .. }
                | Self::Backspace
                | Self::Delete
                | Self::Home
                | Self::End
                | Self::ArrowLeft
                | Self::ArrowRight
                | Self::FocusChanged { .. }
                | Self::KeyboardCaptureDenied
                | Self::ContextMenuResult { .. }
                | Self::SelectLeft
                | Self::SelectRight
                | Self::SelectHome
                | Self::SelectEnd
                | Self::SelectAll
                | Self::FileDragExited
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
    keyboard_capture_request: Option<bool>,
    focus_visible: bool,
    modifiers: KeyModifiers,
    pointer_press_intercepted: bool,
    pub(crate) input_serial: u64,
    window_interaction: core::cell::Cell<Option<(u64, crate::decoration::WindowInteraction)>>,
    command_requests: Vec<CommandId>,
    command_statuses: &'a [CommandStatus],
    command_target: Option<Point>,
    corner_radii: Vec<f32>,
    pane_container: Option<Rect>,
}

impl<'a> EventContext<'a> {
    pub(crate) fn pane_container(&self) -> Option<Rect> {
        self.pane_container
    }

    pub(crate) fn set_pane_container(&mut self, bounds: Option<Rect>) -> Option<Rect> {
        std::mem::replace(&mut self.pane_container, bounds)
    }

    /// Borrows authorization for this dispatch's actual primary press.
    /// The token cannot outlive the event context or be constructed by apps.
    pub fn window_interaction_token(
        &self,
    ) -> Option<crate::decoration::WindowInteractionToken<'_>> {
        (self.input_serial != 0).then_some(crate::decoration::WindowInteractionToken {
            serial: self.input_serial,
            request: &self.window_interaction,
        })
    }

    /// Requests a move only while handling an authenticated primary press.
    /// Returns false when no active token is present or it was already used.
    pub fn request_window_move(&mut self) -> bool {
        self.request_window_interaction(crate::decoration::WindowInteraction::Move)
    }

    /// Requests an input-authorized resize. The platform also enforces the
    /// window's resizable property and consumes the press token once.
    pub fn request_window_resize(&mut self, edge: crate::decoration::ResizeEdge) -> bool {
        self.request_window_interaction(crate::decoration::WindowInteraction::Resize(edge))
    }

    fn request_window_interaction(
        &mut self,
        interaction: crate::decoration::WindowInteraction,
    ) -> bool {
        if self.input_serial == 0 || self.window_interaction.get().is_some() {
            return false;
        }
        self.window_interaction
            .set(Some((self.input_serial, interaction)));
        self.input_serial = 0;
        true
    }

    pub(crate) fn take_window_interaction(
        &mut self,
    ) -> Option<(u64, crate::decoration::WindowInteraction)> {
        let request = self.window_interaction.take();
        if request.is_some() {
            self.input_serial = 0;
        }
        request
    }

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
            keyboard_capture_request: None,
            focus_visible: true,
            modifiers: KeyModifiers::default(),
            pointer_press_intercepted: false,
            input_serial: 0,
            window_interaction: core::cell::Cell::new(None),
            command_requests: Vec::new(),
            command_statuses: &[],
            command_target: None,
            corner_radii: Vec::new(),
            pane_container: None,
        }
    }

    pub(crate) fn set_command_context(
        &mut self,
        command_statuses: &'a [CommandStatus],
        command_target: Option<Point>,
    ) {
        self.command_statuses = command_statuses;
        self.command_target = command_target;
    }

    /// Returns the first responder's published state for a command.
    pub fn command_status(&self, command: CommandId) -> Option<&CommandStatus> {
        self.command_statuses.iter().find(|status| {
            status.command == command
                && self
                    .command_target
                    .is_none_or(|target| status.bounds.contains(target))
        })
    }

    pub(crate) fn inherited_corner_radius(&self) -> Option<f32> {
        self.corner_radii.last().copied()
    }
    pub(crate) fn push_corner_radius(&mut self, radius: f32) {
        self.corner_radii.push(radius);
    }
    pub(crate) fn pop_corner_radius(&mut self) {
        self.corner_radii.pop();
    }

    /// Current keyboard modifiers, also available during pointer interactions.
    pub fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    pub fn typography(&self) -> &Typography {
        self.typography
    }

    /// Measures event-time geometry using the same font scale and cached font
    /// metrics as painting, including for popover and context-menu anchors.
    pub fn measure_context(&mut self) -> crate::view::MeasureContext<'_> {
        crate::view::MeasureContext {
            theme: self.theme,
            typography: self.typography,
            text_measurer: &mut *self.text_measurer,
        }
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

    /// キーボード操作に対応するフォーカス表示が必要かを返します。
    /// フォーカスの所有と、ポインター操作時の枠表示を区別するために使います。
    pub fn focus_visible(&self) -> bool {
        self.focus_visible
    }

    pub fn clear_keyboard_focus(&mut self) {
        self.keyboard_focus_request = Some(None);
    }

    /// Requests temporary platform keyboard ownership for a shell panel.
    /// The platform verifies authority and may emit `KeyboardCaptureDenied`.
    /// Release when the panel closes; focus preemption also cancels capture.
    pub fn set_keyboard_capture(&mut self, captured: bool) {
        self.keyboard_capture_request = Some(captured);
    }

    pub(crate) fn take_keyboard_capture_request(&mut self) -> Option<bool> {
        self.keyboard_capture_request.take()
    }

    pub(crate) fn intercept_pointer_press(&mut self) {
        self.pointer_press_intercepted = true;
    }

    fn take_pointer_press_interception(&mut self) -> bool {
        std::mem::take(&mut self.pointer_press_intercepted)
    }

    /// Queues a semantic command for responder-chain dispatch after the
    /// current event finishes propagating.
    pub fn dispatch_command(&mut self, command: CommandId) {
        self.command_requests.push(command);
    }

    pub(crate) fn take_command_requests(&mut self) -> Vec<CommandId> {
        std::mem::take(&mut self.command_requests)
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
    focused_identity: Option<crate::accessibility::AccessibilityIdentity>,
    focus_nodes: Vec<(Rect, Option<crate::accessibility::AccessibilityIdentity>)>,
    focus_scopes: Vec<FocusScopeState>,
    pending_focus_request: Option<Option<Rect>>,
    window_focused: bool,
    focus_visible: bool,
    modifiers: KeyModifiers,
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

    pub(crate) fn command_target(&self) -> Option<Point> {
        self.focused_bounds.map(|focused| {
            Point::new(
                focused.origin.x + focused.size.width * 0.5,
                focused.origin.y + focused.size.height * 0.5,
            )
        })
    }

    /// キーボード移動の候補を更新します。未フォーカス状態を維持し、
    /// 消えたコントロールから別のコントロールへ自動で移動しません。
    /// ダイアログのフォーカス範囲への移動と明示的な要求は別に扱います。
    pub fn set_accessibility_nodes(&mut self, nodes: &[AccessibilityNode]) {
        let previous_identity = self.focused_identity;
        let previous_scopes = self
            .focus_scopes
            .iter()
            .map(|s| s.bounds)
            .collect::<Vec<_>>();
        self.focus_nodes.clear();
        self.focus_nodes.extend(
            nodes
                .iter()
                .filter(|n| n.focusable && n.enabled)
                .map(|n| (n.bounds, n.identity)),
        );

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

        for (scope_position, (scope_index, bounds)) in
            scope_entries.iter().enumerate().skip(common_scope_count)
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
            self.focused_identity = target.and_then(|b| {
                self.focus_nodes
                    .iter()
                    .find_map(|(n, id)| (*n == b).then_some(*id))
                    .flatten()
            });
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

        if !self.window_focused {
            self.focused_identity = None;
            self.focused_bounds = None;
            self.pending_focus_request = None;
            return;
        }

        if previous_scopes
            == self
                .focus_scopes
                .iter()
                .map(|s| s.bounds)
                .collect::<Vec<_>>()
        {
            if let Some(id) = previous_identity {
                let next = self
                    .focus_nodes
                    .iter()
                    .find_map(|(b, i)| (*i == Some(id)).then_some(*b));
                if next != self.focused_bounds {
                    self.focused_bounds = next;
                    self.pending_focus_request = Some(next);
                }
            }
        }
        self.focused_identity = self.focused_bounds.and_then(|b| {
            self.focus_nodes
                .iter()
                .find_map(|(n, id)| (*n == b).then_some(*id))
                .flatten()
        });
        // None is an intentional state (startup, blank click, or explicit clear).
        // A disappearing control loses focus; it must not select a sibling.
        if self
            .focused_bounds
            .is_some_and(|focused| !self.focus_order.contains(&focused))
        {
            self.focused_bounds = None;
            self.pending_focus_request = Some(None);
        }
    }

    fn next_focus(&self, backwards: bool) -> Option<Rect> {
        let count = self.focus_order.len();
        if count == 0 {
            return None;
        }

        let current = self.focused_bounds.and_then(|focused| {
            self.focus_order
                .iter()
                .position(|bounds| *bounds == focused)
        });
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
        match event {
            PlatformEvent::ModifiersChanged { modifiers }
            | PlatformEvent::KeyPressed { modifiers, .. }
            | PlatformEvent::KeyReleased { modifiers, .. } => self.modifiers = *modifiers,
            PlatformEvent::Focused(false) => self.modifiers = KeyModifiers::default(),
            _ => {}
        }
        context.modifiers = self.modifiers;
        context.focus_visible = self.focus_visible;

        if let PlatformEvent::Focused(focused) = event {
            self.window_focused = *focused;
            if !focused {
                self.pointer_position = None;
                self.focused_bounds = None;
                self.focused_identity = None;
                self.pending_focus_request = None;
            }
        }

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

            self.focus_visible = true;
            context.focus_visible = true;
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::FocusVisibilityChanged { visible: true },
                context,
            ));
            let target = self.next_focus(modifiers.shift());
            self.focused_bounds = target;
            self.focused_identity = target.and_then(|b| {
                self.focus_nodes
                    .iter()
                    .find_map(|(n, id)| (*n == b).then_some(*id))
                    .flatten()
            });
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
            self.focus_visible = false;
            context.focus_visible = false;
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::FocusVisibilityChanged { visible: false },
                context,
            ));
            let focus_result = root.handle_event(
                bounds,
                &ViewEvent::PointerFocusRequested { position },
                context,
            );
            result = result.merge(focus_result);
            if context.take_pointer_press_interception() {
                return result;
            }
            let target = self
                .focus_order
                .iter()
                .copied()
                .find(|bounds| bounds.contains(position));
            self.focused_bounds = target;
            self.focused_identity = target.and_then(|b| {
                self.focus_nodes
                    .iter()
                    .find_map(|(n, id)| (*n == b).then_some(*id))
                    .flatten()
            });
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::KeyboardFocusRequested { bounds: target },
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
            self.focused_identity = target.and_then(|b| {
                self.focus_nodes
                    .iter()
                    .find_map(|(n, id)| (*n == b).then_some(*id))
                    .flatten()
            });
            result = result.merge(root.handle_event(
                bounds,
                &ViewEvent::KeyboardFocusRequested { bounds: target },
                context,
            ));
        }

        result
    }

    /// Routes a command through the branch containing keyboard focus. Views
    /// on that branch receive it from the deepest descendant back outward.
    pub fn dispatch_command(
        &mut self,
        root: &dyn View,
        bounds: Rect,
        command: CommandId,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let target = self.command_target();
        root.handle_event(bounds, &ViewEvent::Command { command, target }, context)
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
            PlatformEvent::KeyReleased { key, modifiers } => Some(ViewEvent::KeyReleased {
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

            PlatformEvent::Focused(focused) => Some(ViewEvent::FocusChanged { focused: *focused }),

            PlatformEvent::ContextMenuResult {
                request_id,
                command_id,
            } => Some(ViewEvent::ContextMenuResult {
                request_id: *request_id,
                command_id: *command_id,
            }),

            PlatformEvent::ModifiersChanged { modifiers } => Some(ViewEvent::ModifiersChanged {
                modifiers: *modifiers,
            }),
            PlatformEvent::TextInput { text } => Some(ViewEvent::TextInput { text: text.clone() }),
            PlatformEvent::FileHovered { path } => Some(ViewEvent::FileDragEntered {
                path: path.clone(),
                position: self.pointer_position,
            }),
            PlatformEvent::FileHoverCancelled => Some(ViewEvent::FileDragExited),
            PlatformEvent::FileDropped { path } => Some(ViewEvent::FileDropped {
                path: path.clone(),
                position: self.pointer_position,
            }),
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

#[cfg(test)]
mod tests {
    use super::{EventContext, EventDispatcher, EventResult, ViewEvent};
    use crate::accessibility::{AccessibilityNode, AccessibilityRole};
    use crate::command::{CommandStatus, standard as commands};
    use crate::geometry::Rect;
    use crate::platform::{ButtonState, PlatformEvent, PointerButton};
    use crate::theme::Theme;
    use crate::typography::TextMeasurer;
    use crate::view::{PaintContext, View};
    use std::cell::Cell;

    #[test]
    fn pointer_context_retains_modifiers_and_clears_them_on_focus_loss() {
        struct Probe(Cell<u8>);
        impl View for Probe {
            fn paint(&self, _: Rect, _: &mut PaintContext<'_>) {}
            fn handle_event(
                &self,
                _: Rect,
                event: &ViewEvent,
                context: &mut EventContext<'_>,
            ) -> EventResult {
                if matches!(event, ViewEvent::PointerPressed { .. }) {
                    self.0.set(context.modifiers().bits());
                }
                EventResult::Ignored
            }
        }
        let probe = Probe(Cell::new(0));
        let theme = Theme::LIGHT;
        let mut measure = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measure);
        let mut dispatcher = EventDispatcher::new();
        let bounds = Rect::new(0., 0., 400., 300.);
        let modifiers = crate::platform::KeyModifiers::from_bits(
            crate::platform::KeyModifiers::CONTROL | crate::platform::KeyModifiers::SHIFT,
        );
        dispatcher.dispatch(
            &probe,
            bounds,
            &PlatformEvent::ModifiersChanged { modifiers },
            &mut context,
        );
        dispatcher.dispatch(
            &probe,
            bounds,
            &PlatformEvent::PointerMoved { x: 10., y: 10. },
            &mut context,
        );
        dispatcher.dispatch(
            &probe,
            bounds,
            &PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            },
            &mut context,
        );
        assert_eq!(probe.0.get(), modifiers.bits());
        dispatcher.dispatch(&probe, bounds, &PlatformEvent::Focused(false), &mut context);
        dispatcher.dispatch(
            &probe,
            bounds,
            &PlatformEvent::PointerMoved { x: 10., y: 10. },
            &mut context,
        );
        dispatcher.dispatch(
            &probe,
            bounds,
            &PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            },
            &mut context,
        );
        assert_eq!(probe.0.get(), 0);
    }

    #[test]
    fn pointer_focus_keeps_keyboard_input_without_showing_keyboard_focus_ring() {
        struct FocusRecorder(Cell<(bool, bool)>);
        impl View for FocusRecorder {
            fn paint(&self, _: Rect, _: &mut PaintContext<'_>) {}
            fn handle_event(
                &self,
                _: Rect,
                event: &ViewEvent,
                context: &mut EventContext<'_>,
            ) -> EventResult {
                if let ViewEvent::KeyboardFocusRequested { bounds } = event {
                    self.0.set((bounds.is_some(), context.focus_visible()));
                }
                EventResult::Ignored
            }
        }
        let root = FocusRecorder(Cell::new((false, false)));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut node = AccessibilityNode::new(AccessibilityRole::Button, bounds);
        node.focusable = true;
        let mut dispatcher = EventDispatcher::new();
        dispatcher.set_accessibility_nodes(&[node]);
        let theme = Theme::current();
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        for event in [
            PlatformEvent::Focused(true),
            PlatformEvent::PointerMoved { x: 20.0, y: 20.0 },
            PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            },
        ] {
            dispatcher.dispatch(&root, bounds, &event, &mut context);
        }
        assert_eq!(root.0.get(), (true, false));
        dispatcher.dispatch(
            &root,
            bounds,
            &PlatformEvent::KeyPressed {
                key: crate::platform::Key::Tab,
                modifiers: crate::platform::KeyModifiers::default(),
            },
            &mut context,
        );
        assert_eq!(root.0.get(), (true, true));
    }

    struct PointerRecorder {
        presses: Cell<usize>,
    }

    impl View for PointerRecorder {
        fn paint(&self, _bounds: Rect, _context: &mut PaintContext<'_>) {}

        fn handle_event(
            &self,
            _bounds: Rect,
            event: &ViewEvent,
            _context: &mut EventContext<'_>,
        ) -> EventResult {
            match event {
                ViewEvent::PointerFocusRequested { .. } => EventResult::Consumed,
                ViewEvent::PointerPressed { .. } => {
                    self.presses.set(self.presses.get() + 1);
                    EventResult::Consumed
                }
                _ => EventResult::Ignored,
            }
        }
    }

    #[test]
    fn startup_does_not_assign_keyboard_focus_to_the_first_control() {
        let bounds = Rect::new(12.0, 24.0, 320.0, 200.0);
        let mut editor = AccessibilityNode::new(AccessibilityRole::TextField, bounds);
        editor.focusable = true;

        let mut dispatcher = EventDispatcher::new();
        dispatcher.window_focused = true;
        dispatcher.set_accessibility_nodes(&[editor]);

        assert_eq!(dispatcher.focused_bounds, None);
        assert_eq!(dispatcher.pending_focus_request, None);
    }

    #[test]
    fn command_validation_prefers_the_focused_descendant() {
        let outer = Rect::new(0.0, 0.0, 400.0, 300.0);
        let focused = Rect::new(20.0, 30.0, 100.0, 40.0);
        let statuses = [
            CommandStatus::new(commands::SAVE, focused, false),
            CommandStatus::new(commands::SAVE, outer, true),
        ];
        let theme = Theme::LIGHT;
        let mut measurer = TextMeasurer::new();
        let mut context = super::EventContext::new(&theme, &theme.typography, &mut measurer);
        context.set_command_context(&statuses, Some(crate::geometry::Point::new(50.0, 50.0)));

        assert_eq!(context.command_status(commands::SAVE), Some(&statuses[0]));
    }

    #[test]
    fn later_controls_do_not_acquire_initial_focus() {
        let bounds = Rect::new(0.0, 0.0, 240.0, 160.0);
        let mut editor = AccessibilityNode::new(AccessibilityRole::TextField, bounds);
        editor.focusable = true;

        let mut dispatcher = EventDispatcher::new();
        dispatcher.window_focused = true;
        dispatcher.set_accessibility_nodes(&[]);
        assert_eq!(dispatcher.pending_focus_request, None);

        dispatcher.set_accessibility_nodes(&[editor]);
        assert_eq!(dispatcher.pending_focus_request, None);
    }

    #[test]
    fn unfocused_window_does_not_focus_its_first_control() {
        let bounds = Rect::new(8.0, 8.0, 120.0, 32.0);
        let mut button = AccessibilityNode::new(AccessibilityRole::Button, bounds);
        button.focusable = true;

        let mut dispatcher = EventDispatcher::new();
        dispatcher.set_accessibility_nodes(&[button]);

        assert_eq!(dispatcher.focused_bounds, None);
        assert_eq!(dispatcher.pending_focus_request, None);
    }

    #[test]
    fn pointer_focus_preflight_does_not_swallow_regular_clicks() {
        let root = PointerRecorder {
            presses: Cell::new(0),
        };
        let bounds = Rect::new(0.0, 0.0, 300.0, 200.0);
        let theme = Theme::LIGHT;
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        let mut dispatcher = EventDispatcher::new();

        dispatcher.dispatch(
            &root,
            bounds,
            &PlatformEvent::PointerMoved { x: 40.0, y: 50.0 },
            &mut context,
        );
        dispatcher.dispatch(
            &root,
            bounds,
            &PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            },
            &mut context,
        );

        assert_eq!(root.presses.get(), 1);
    }
}

#[cfg(test)]
mod focus_modality_regressions {
    use super::*;
    use std::cell::Cell;
    struct Probe {
        visible: Cell<bool>,
        consume_tab: bool,
    }
    impl View for Probe {
        fn paint(&self, _: Rect, _: &mut crate::view::PaintContext<'_>) {}
        fn handle_event(&self, _: Rect, e: &ViewEvent, _: &mut EventContext<'_>) -> EventResult {
            if let ViewEvent::FocusVisibilityChanged { visible } = e {
                self.visible.set(*visible);
            }
            if self.consume_tab && matches!(e, ViewEvent::KeyPressed { key: Key::Tab, .. }) {
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        }
    }
    #[test]
    fn typing_shortcuts_and_editor_tabs_do_not_show_unrelated_focus_rings() {
        let theme = crate::theme::Theme::LIGHT;
        let mut text = crate::typography::TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        let b = Rect::new(0., 0., 100., 40.);
        let p = Probe {
            visible: Cell::new(false),
            consume_tab: true,
        };
        let mut d = EventDispatcher::new();
        for (key, modifiers) in [
            (Key::Character('a'), KeyModifiers::default()),
            (
                Key::Character('t'),
                KeyModifiers::from_bits(KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            ),
            (Key::Tab, KeyModifiers::default()),
        ] {
            d.dispatch(
                &p,
                b,
                &PlatformEvent::KeyPressed { key, modifiers },
                &mut cx,
            );
            assert!(!p.visible.get());
            assert!(!cx.focus_visible());
        }
        let p = Probe {
            visible: Cell::new(false),
            consume_tab: false,
        };
        d.dispatch(
            &p,
            b,
            &PlatformEvent::KeyPressed {
                key: Key::Tab,
                modifiers: KeyModifiers::default(),
            },
            &mut cx,
        );
        assert!(p.visible.get());
        assert!(cx.focus_visible());
    }
}

#[cfg(test)]
mod unfocused_state_tests {
    use super::*;
    use crate::accessibility::AccessibilityRole;
    use crate::{
        components::TextField, components::TextFieldInteractionState, draw_command::DisplayList,
        theme::Theme, typography::TextMeasurer, view::PaintContext,
    };
    #[test]
    fn clicking_blank_space_clears_a_real_text_field_across_repaint_and_activation() {
        let state = TextFieldInteractionState::new();
        let field = TextField::with_interaction(state.clone());
        let theme = Theme::LIGHT;
        let bounds = Rect::new(0., 0., 300., 200.);
        let control = Rect::new(0., 0., 200., 32.);
        let mut text = TextMeasurer::new();
        let mut nodes = vec![];
        let mut list = DisplayList::new();
        field.paint(
            control,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text)
                .with_accessibility_nodes(&mut nodes),
        );
        struct Root<'a>(&'a TextField, Rect);
        impl View for Root<'_> {
            fn paint(&self, _: Rect, _: &mut PaintContext<'_>) {}
            fn handle_event(
                &self,
                _: Rect,
                e: &ViewEvent,
                c: &mut EventContext<'_>,
            ) -> EventResult {
                self.0.handle_event(self.1, e, c)
            }
        }
        let root = Root(&field, control);
        let mut d = EventDispatcher::new();
        d.set_accessibility_nodes(&nodes);
        let mut c = EventContext::new(&theme, &theme.typography, &mut text);
        d.dispatch(&root, bounds, &PlatformEvent::Focused(true), &mut c);
        d.set_accessibility_nodes(&nodes);
        assert!(!state.is_focused());
        for (x, y) in [(10., 10.), (250., 150.)] {
            d.dispatch(&root, bounds, &PlatformEvent::PointerMoved { x, y }, &mut c);
            d.dispatch(
                &root,
                bounds,
                &PlatformEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Pressed,
                },
                &mut c,
            );
            d.dispatch(
                &root,
                bounds,
                &PlatformEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Released,
                },
                &mut c,
            );
            assert_eq!(state.is_focused(), x < 200.);
            if x < 200. {
                d.dispatch(
                    &root,
                    bounds,
                    &PlatformEvent::TextInput {
                        text: "kept".into(),
                    },
                    &mut c,
                );
                assert_eq!(state.value(), "kept");
            }
        }
        d.set_accessibility_nodes(&nodes);
        d.dispatch(
            &root,
            bounds,
            &PlatformEvent::TextInput {
                text: "must not type".into(),
            },
            &mut c,
        );
        assert_eq!(state.value(), "kept");
        assert_eq!(d.focused_bounds, None);
        assert!(!state.is_focused());
        d.dispatch(&root, bounds, &PlatformEvent::Focused(false), &mut c);
        d.dispatch(&root, bounds, &PlatformEvent::Focused(true), &mut c);
        d.set_accessibility_nodes(&nodes);
        assert_eq!(d.focused_bounds, None);
        d.dispatch(
            &root,
            bounds,
            &PlatformEvent::KeyPressed {
                key: Key::Tab,
                modifiers: KeyModifiers::default(),
            },
            &mut c,
        );
        assert_eq!(d.focused_bounds, Some(control));
        assert!(state.is_focused());
    }
    #[test]
    fn disappearing_control_and_modal_restore_do_not_focus_an_unrelated_control() {
        let a = Rect::new(0., 0., 100., 30.);
        let b = Rect::new(0., 40., 100., 30.);
        let mut first = AccessibilityNode::new(AccessibilityRole::Button, a);
        first.focusable = true;
        let mut second = AccessibilityNode::new(AccessibilityRole::Button, b);
        second.focusable = true;
        let mut d = EventDispatcher::new();
        d.window_focused = true;
        d.focused_bounds = Some(a);
        d.set_accessibility_nodes(&[second.clone()]);
        assert_eq!(d.focused_bounds, None);
        assert_eq!(d.pending_focus_request, Some(None));
        let mut scope =
            AccessibilityNode::new(AccessibilityRole::Dialog, Rect::new(0., 0., 200., 200.));
        scope.focus_scope = true;
        d.set_accessibility_nodes(&[scope, second.clone()]);
        assert_eq!(d.focused_bounds, Some(b));
        d.set_accessibility_nodes(&[first, second]);
        assert_eq!(d.focused_bounds, None);
        assert_eq!(d.pending_focus_request, Some(None));
    }
}

#[cfg(test)]
mod identified_focus_tests {
    use super::*;
    #[test]
    fn focused_identity_follows_reorder_and_is_not_transferred_to_a_replacement() {
        let a = Rect::new(0.0, 0.0, 100.0, 40.0);
        let b = Rect::new(100.0, 0.0, 100.0, 40.0);
        let mut first =
            AccessibilityNode::new(crate::accessibility::AccessibilityRole::TextField, a);
        first.identity =
            Some(crate::accessibility::AccessibilityIdentity::Component { id: 1, element: 0 });
        first.focusable = true;
        let mut second = first.clone();
        second.identity =
            Some(crate::accessibility::AccessibilityIdentity::Component { id: 2, element: 0 });
        second.bounds = b;
        let mut d = EventDispatcher::new();
        d.window_focused = true;
        d.focused_bounds = Some(a);
        d.focused_identity =
            Some(crate::accessibility::AccessibilityIdentity::Component { id: 1, element: 0 });
        first.bounds = b;
        second.bounds = a;
        d.set_accessibility_nodes(&[first, second.clone()]);
        assert_eq!(d.focused_bounds, Some(b));
        assert_eq!(
            d.focused_identity,
            Some(crate::accessibility::AccessibilityIdentity::Component { id: 1, element: 0 })
        );
        second.bounds = b;
        d.set_accessibility_nodes(&[second]);
        assert_eq!(d.focused_bounds, None);
        assert_eq!(d.focused_identity, None);
    }
}
