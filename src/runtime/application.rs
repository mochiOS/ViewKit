#![allow(clippy::missing_const_for_thread_local)]

//! ViewKitアプリケーションとプラットフォームバックエンドをガッッッッタイ！します

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::app::{App, ViewContext, WindowId};
use crate::appearance::AppearanceSettings;
use crate::command::CommandStatus;
use crate::components::{BorderStyle, Ellipse, EllipseColor, Rectangle, RectangleColor, Text};
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{ContextMenuRequest, EventContext, EventDispatcher, RedrawRequest};
use crate::geometry::{Point, Rect};
use crate::platform::{
    ButtonState, Key, PlatformApplication, PlatformEvent, PlatformWindow, PlatformWindowCommand,
    PointerButton, WindowConfig,
};
use crate::renderer::Viewport;
use crate::state::take_state_changed;
use crate::theme::{ShadowStyle, Theme};
use crate::typography::TextMeasurer;
use crate::view::{PaintContext, RedrawSchedule, View};

thread_local! {
    static EXIT_REQUESTED: Cell<bool> = const { Cell::new(false) };
    static WINDOW_REQUESTS: RefCell<Vec<WindowRequest>> = const { RefCell::new(Vec::new()) };
    static KEY_WINDOW: Cell<Option<WindowId>> = const { Cell::new(None) };
    static MAIN_WINDOW: Cell<Option<WindowId>> = const { Cell::new(None) };
}

static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(WindowId::PRIMARY.raw() + 1);

enum WindowRequest {
    Open(WindowId),
    Alert {
        id: WindowId,
        title: String,
        message: String,
        on_dismiss: Box<dyn FnOnce()>,
    },
    Close(WindowId),
    RequestClose(WindowId),
}

/// 現在のViewKitアプリケーションへ正常終了を要求します。
///
/// プラットフォームのイベントループを抜け、WindowやSurfaceを破棄してから
/// [`run`]を返します。
pub fn request_exit() {
    EXIT_REQUESTED.with(|requested| requested.set(true));
}

/// Requests a new application window and returns its stable identity.
///
/// The application's [`App::window_for`] and [`App::body_for`] hooks provide
/// the configuration and content when the platform creates the window.
pub fn request_new_window() -> WindowId {
    let raw = NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed).max(2);
    let id = WindowId::from_raw(raw).expect("allocated window IDs are non-zero");
    WINDOW_REQUESTS.with(|requests| requests.borrow_mut().push(WindowRequest::Open(id)));
    id
}

/// Presents a compact, frontmost error window managed by the ViewKit runtime.
///
/// Unlike an application-defined window, this window does not require the
/// application's `Body` type to represent alert content.
pub fn request_alert_window(
    title: impl Into<String>,
    message: impl Into<String>,
    on_dismiss: impl FnOnce() + 'static,
) -> WindowId {
    let raw = NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed).max(2);
    let id = WindowId::from_raw(raw).expect("allocated window IDs are non-zero");
    WINDOW_REQUESTS.with(|requests| {
        requests.borrow_mut().push(WindowRequest::Alert {
            id,
            title: title.into(),
            message: message.into(),
            on_dismiss: Box::new(on_dismiss),
        });
    });
    id
}

/// Requests that a specific application window close.
pub fn request_close_window(window: WindowId) {
    WINDOW_REQUESTS.with(|requests| {
        requests
            .borrow_mut()
            .push(WindowRequest::RequestClose(window));
    });
}

/// Closes a window after the application has already approved the operation.
/// Most callers should use [`request_close_window`].
pub fn close_window(window: WindowId) {
    WINDOW_REQUESTS.with(|requests| requests.borrow_mut().push(WindowRequest::Close(window)));
}

/// Requests that the current key window close.
pub fn request_close_key_window() {
    if let Some(window) = key_window().or_else(main_window) {
        request_close_window(window);
    }
}

/// Returns the window currently receiving keyboard input.
pub fn key_window() -> Option<WindowId> {
    KEY_WINDOW.with(Cell::get)
}

/// Returns the application's main document window.
pub fn main_window() -> Option<WindowId> {
    MAIN_WINDOW.with(Cell::get)
}

fn exit_requested() -> bool {
    EXIT_REQUESTED.with(Cell::get)
}

fn reset_exit_request() {
    EXIT_REQUESTED.with(|requested| requested.set(false));
    WINDOW_REQUESTS.with(|requests| requests.borrow_mut().clear());
    KEY_WINDOW.with(|window| window.set(None));
    MAIN_WINDOW.with(|window| window.set(None));
}

/// `App`をプラットフォームバックエンド上で実行するランタイムです。
pub(crate) struct ApplicationRuntime<A>
where
    A: App,
{
    app: A,
    theme: Theme,
    appearance: AppearanceSettings,
    windows: BTreeMap<WindowId, WindowRuntime<A::Body>>,
    pending_window_commands: Vec<PlatformWindowCommand>,
}

struct WindowRuntime<Body> {
    root: Option<Body>,
    alert: Option<RuntimeAlert>,
    viewport: Option<Viewport>,
    text_measurer: TextMeasurer,
    accessibility_nodes: Vec<AccessibilityNode>,
    command_statuses: Vec<CommandStatus>,
    title: String,
    event_dispatcher: EventDispatcher,
    redraw_schedule: RedrawSchedule,
    pending_redraw: RedrawRequest,
    fallback_context_menu: Option<FallbackContextMenu>,
}

struct RuntimeAlert {
    title: String,
    message: String,
    hovered: bool,
    pressed: bool,
    on_dismiss: Option<Box<dyn FnOnce()>>,
}

impl<Body> WindowRuntime<Body> {
    fn new(title: String, appearance: &AppearanceSettings) -> Self {
        let mut text_measurer = TextMeasurer::new();
        text_measurer.set_font_scale(appearance.font_scale());
        Self {
            root: None,
            alert: None,
            viewport: None,
            text_measurer,
            accessibility_nodes: Vec::new(),
            command_statuses: Vec::new(),
            title,
            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),
            pending_redraw: RedrawRequest::None,
            fallback_context_menu: None,
        }
    }

    fn new_alert(
        title: String,
        message: String,
        on_dismiss: Box<dyn FnOnce()>,
        appearance: &AppearanceSettings,
    ) -> Self {
        let mut window = Self::new(String::from("Error"), appearance);
        window.alert = Some(RuntimeAlert {
            title,
            message,
            hovered: false,
            pressed: false,
            on_dismiss: Some(on_dismiss),
        });
        window
    }
}

struct FallbackContextMenu {
    request: ContextMenuRequest,
    pointer: Option<Point>,
}

impl<A> ApplicationRuntime<A>
where
    A: App,
{
    pub(crate) fn new(app: A) -> Self {
        reset_exit_request();
        let window_title = app.window_for(WindowId::PRIMARY).title().to_owned();
        let appearance = AppearanceSettings::load();
        let theme = appearance.theme();
        Theme::set_current(theme);
        let mut windows = BTreeMap::new();
        windows.insert(
            WindowId::PRIMARY,
            WindowRuntime::new(window_title, &appearance),
        );
        MAIN_WINDOW.with(|window| window.set(Some(WindowId::PRIMARY)));
        Self {
            app,
            theme,
            appearance,
            windows,
            pending_window_commands: Vec::new(),
        }
    }

    fn ensure_window(&mut self, id: WindowId) {
        if self.windows.contains_key(&id) {
            return;
        }
        let title = self.app.window_for(id).title().to_owned();
        self.windows
            .insert(id, WindowRuntime::new(title, &self.appearance));
    }

    fn rebuild_root(&mut self, id: WindowId, viewport: Viewport) {
        self.rebuild_root_with_redraw(id, viewport, RedrawRequest::Full);
    }

    fn rebuild_root_with_redraw(
        &mut self,
        id: WindowId,
        viewport: Viewport,
        redraw: RedrawRequest,
    ) {
        self.ensure_window(id);
        if self
            .windows
            .get(&id)
            .is_some_and(|state| state.alert.is_some())
        {
            let state = self.windows.get_mut(&id).expect("window was ensured");
            state.viewport = Some(viewport);
            state.pending_redraw = redraw;
            return;
        }
        let context = ViewContext::new(viewport);
        Theme::set_current(self.theme);
        let root = self.app.body_for(id, &context);
        let state = self.windows.get_mut(&id).expect("window was ensured");
        state.root = Some(root);
        state.viewport = Some(viewport);
        state.pending_redraw = redraw;

        let _ = take_state_changed();
    }

    fn ensure_root(&mut self, id: WindowId, viewport: Viewport) {
        self.ensure_window(id);
        let state = self.windows.get(&id).expect("window was ensured");
        if (state.root.is_none() && state.alert.is_none()) || state.viewport != Some(viewport) {
            self.rebuild_root(id, viewport);
        }
    }

    fn invalidate_other_windows(&mut self, current: WindowId) {
        for (&id, state) in &mut self.windows {
            if id == current {
                continue;
            }
            state.root = None;
            state.pending_redraw = RedrawRequest::Full;
            self.pending_window_commands
                .push(PlatformWindowCommand::Redraw { id });
        }
    }
}

impl<A> PlatformApplication for ApplicationRuntime<A>
where
    A: App,
{
    fn reopen(&mut self) {
        self.app.reopened();
        if self.windows.is_empty() {
            self.ensure_window(WindowId::PRIMARY);
            MAIN_WINDOW.with(|window| window.set(Some(WindowId::PRIMARY)));
            self.pending_window_commands
                .push(PlatformWindowCommand::Open {
                    id: WindowId::PRIMARY,
                    config: window_config(self.app.window_for(WindowId::PRIMARY)),
                });
        }
    }

    fn handle_platform_message(&mut self, message: &[u8]) -> bool {
        let handled = self.app.handle_platform_message(message);
        if handled && take_state_changed() {
            for (&id, state) in &mut self.windows {
                state.root = None;
                state.pending_redraw = RedrawRequest::Full;
                self.pending_window_commands
                    .push(PlatformWindowCommand::Redraw { id });
            }
        }
        handled
    }

    fn take_window_commands(&mut self) -> Vec<PlatformWindowCommand> {
        let requests =
            WINDOW_REQUESTS.with(|requests| requests.borrow_mut().drain(..).collect::<Vec<_>>());
        for request in requests {
            match request {
                WindowRequest::Open(id) => {
                    self.ensure_window(id);
                    self.pending_window_commands
                        .push(PlatformWindowCommand::Open {
                            id,
                            config: window_config(self.app.window_for(id)),
                        });
                }
                WindowRequest::Alert {
                    id,
                    title,
                    message,
                    on_dismiss,
                } => {
                    let window_title = title.clone();
                    self.windows.insert(
                        id,
                        WindowRuntime::new_alert(title, message, on_dismiss, &self.appearance),
                    );
                    self.pending_window_commands
                        .push(PlatformWindowCommand::Open {
                            id,
                            config: WindowConfig {
                                title: window_title,
                                size: crate::geometry::Size::new(520.0, 190.0),
                                resizable: false,
                                fullscreen: false,
                                secure_overlay: false,
                                system_modal: false,
                            },
                        });
                }
                WindowRequest::Close(id) => {
                    self.remove_window(id);
                    self.pending_window_commands
                        .push(PlatformWindowCommand::Close { id });
                }
                WindowRequest::RequestClose(id) => {
                    if self.windows.contains_key(&id) {
                        self.pending_window_commands
                            .push(PlatformWindowCommand::RequestClose { id });
                    }
                }
            }
        }
        std::mem::take(&mut self.pending_window_commands)
    }

    fn handle_event(&mut self, mut event: PlatformEvent, window: &dyn PlatformWindow) {
        let window_id = window.id();
        let alert_window = self
            .windows
            .get(&window_id)
            .is_some_and(|state| state.alert.is_some());
        if let PlatformEvent::Focused(focused) = event {
            if focused {
                KEY_WINDOW.with(|key| key.set(Some(window_id)));
                if !alert_window {
                    MAIN_WINDOW.with(|main| main.set(Some(window_id)));
                }
            } else {
                KEY_WINDOW.with(|key| {
                    if key.get() == Some(window_id) {
                        key.set(None);
                    }
                });
            }
        }
        match &event {
            PlatformEvent::Resumed { viewport }
            | PlatformEvent::Resized { viewport }
            | PlatformEvent::ScaleFactorChanged { viewport } => {
                self.rebuild_root(window_id, *viewport);
                return;
            }

            PlatformEvent::CloseRequested => {
                let _ = self.should_close_window(window);
                return;
            }

            PlatformEvent::RedrawRequested => {
                return;
            }

            _ => {}
        }

        let viewport = window.viewport();

        self.ensure_root(window_id, viewport);

        if alert_window {
            self.handle_alert_event(window_id, &event, window);
            return;
        }

        let state = self
            .windows
            .get_mut(&window_id)
            .expect("window was ensured");
        if let Some(menu) = state.fallback_context_menu.as_mut() {
            match event.clone() {
                PlatformEvent::PointerMoved { x, y } => {
                    menu.pointer = Some(Point::new(x, y));
                    state.pending_redraw = RedrawRequest::Full;
                    window.request_redraw();
                    return;
                }
                PlatformEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Pressed,
                } => return,
                PlatformEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Released,
                } => {
                    let request_id = menu.request.request_id;
                    let command_id = menu.pointer.and_then(|position| {
                        fallback_menu_command_at(
                            &menu.request,
                            position,
                            viewport.logical_bounds(),
                            &self.theme,
                        )
                    });
                    state.fallback_context_menu = None;
                    event = PlatformEvent::ContextMenuResult {
                        request_id,
                        command_id,
                    };
                }
                PlatformEvent::KeyPressed {
                    key: Key::Escape, ..
                }
                | PlatformEvent::Focused(false) => {
                    let request_id = menu.request.request_id;
                    state.fallback_context_menu = None;
                    event = PlatformEvent::ContextMenuResult {
                        request_id,
                        command_id: None,
                    };
                }
                _ => return,
            }
        }

        let (redraw_request, cursor_icon, context_menu_request) = {
            let root = state
                .root
                .as_ref()
                .expect("root view must exist after ensure_root");

            let mut context = EventContext::new(
                &self.theme,
                &self.theme.typography,
                &mut state.text_measurer,
            );
            context.set_command_context(
                &state.command_statuses,
                state.event_dispatcher.command_target(),
            );

            state
                .event_dispatcher
                .dispatch(root, viewport.logical_bounds(), &event, &mut context);

            let mut commands = context.take_command_requests();
            let mut dispatched = 0usize;
            while let Some(command) = commands.pop() {
                if dispatched == 64 {
                    break;
                }
                dispatched += 1;
                state.event_dispatcher.dispatch_command(
                    root,
                    viewport.logical_bounds(),
                    command,
                    &mut context,
                );
                commands.extend(context.take_command_requests());
            }

            (
                context.redraw_request(),
                context.cursor_icon(),
                context.take_context_menu_request(),
            )
        };

        if let Some(cursor_icon) = cursor_icon {
            window.set_cursor(cursor_icon);
        }
        if let Some(request) = context_menu_request {
            if !window.show_context_menu(&request) {
                state.fallback_context_menu = Some(FallbackContextMenu {
                    request,
                    pointer: state.event_dispatcher.pointer_position(),
                });
                state.pending_redraw = RedrawRequest::Full;
                window.request_redraw();
            }
        }

        let state_changed = take_state_changed();
        let redraw_request = redraw_after_event(state_changed, redraw_request);

        if state_changed {
            self.rebuild_root_with_redraw(window_id, viewport, redraw_request);
            self.invalidate_other_windows(window_id);
        } else {
            state.pending_redraw = state.pending_redraw.merge(redraw_request);
        }

        if state_changed || redraw_request.is_requested() {
            window.request_redraw();
        }

        let window_title = self.app.window_for(window.id()).title().to_owned();
        let state = self
            .windows
            .get_mut(&window_id)
            .expect("window was ensured");
        if window_title != state.title {
            window.set_title(&window_title);
            state.title = window_title;
        }
    }

    fn draw(&mut self, window: &dyn PlatformWindow, display_list: &mut DisplayList) -> Rect {
        let window_id = window.id();
        let viewport = window.viewport();
        self.ensure_root(window_id, viewport);
        if take_state_changed() {
            self.rebuild_root(window_id, viewport);
            self.invalidate_other_windows(window_id);
        }

        let state = self
            .windows
            .get_mut(&window_id)
            .expect("window was ensured");
        let viewport_bounds = viewport.logical_bounds();
        let scheduled_redraw = state.redraw_schedule.take_due(Instant::now());
        state.pending_redraw = state.pending_redraw.merge(scheduled_redraw);

        let dirty_bounds = match std::mem::take(&mut state.pending_redraw) {
            RedrawRequest::Region(bounds) => bounds
                .intersection(viewport_bounds)
                .unwrap_or(viewport_bounds),

            RedrawRequest::None | RedrawRequest::Full => viewport_bounds,
        };

        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        state.redraw_schedule.clear();
        state.accessibility_nodes.clear();
        state.command_statuses.clear();

        let mut context = PaintContext::new(
            display_list,
            &self.theme,
            &self.theme.typography,
            &mut state.text_measurer,
        )
        .with_redraw_schedule(&mut state.redraw_schedule)
        .with_accessibility_nodes(&mut state.accessibility_nodes)
        .with_command_statuses(&mut state.command_statuses);

        if let Some(alert) = state.alert.as_ref() {
            paint_alert_window(alert, viewport_bounds, &mut context);
        } else {
            let root = state
                .root
                .as_ref()
                .expect("root view must exist after ensure_root");
            root.paint(viewport_bounds, &mut context);
            if let Some(menu) = &state.fallback_context_menu {
                paint_fallback_context_menu(menu, viewport_bounds, &mut context);
            }
        }
        drop(context);
        state
            .event_dispatcher
            .set_accessibility_nodes(&state.accessibility_nodes);
        window.update_accessibility(&state.accessibility_nodes);

        dirty_bounds
    }

    fn next_redraw_at(&self, window: WindowId) -> Option<Instant> {
        self.windows
            .get(&window)
            .and_then(|state| state.redraw_schedule.deadline())
    }

    fn accessibility_nodes(&self, window: WindowId) -> &[AccessibilityNode] {
        self.windows
            .get(&window)
            .map_or(&[], |state| state.accessibility_nodes.as_slice())
    }

    fn reload_appearance(&mut self) -> bool {
        let appearance = AppearanceSettings::load();
        if appearance == self.appearance {
            return false;
        }
        let theme = appearance.theme();
        let font_scale = appearance.font_scale();
        self.appearance = appearance;
        self.theme = theme;
        Theme::set_current(self.theme);
        self.app.appearance_changed();
        for (&id, state) in &mut self.windows {
            state.text_measurer.set_font_scale(font_scale);
            state.root = None;
            state.pending_redraw = RedrawRequest::Full;
            self.pending_window_commands
                .push(PlatformWindowCommand::Redraw { id });
        }
        true
    }

    fn interface_scale_factor(&self) -> f64 {
        self.appearance.ui_scale()
    }

    fn exit_requested(&self) -> bool {
        exit_requested()
    }

    fn should_close_window(&mut self, window: &dyn PlatformWindow) -> bool {
        if self
            .windows
            .get(&window.id())
            .is_some_and(|state| state.alert.is_some())
        {
            self.remove_window(window.id());
            return true;
        }
        if self.app.close_requested_for_window(window.id()) {
            self.remove_window(window.id());
            true
        } else {
            self.ensure_window(window.id());
            self.windows
                .get_mut(&window.id())
                .expect("window was ensured")
                .pending_redraw = RedrawRequest::Full;
            window.request_redraw();
            false
        }
    }
}

impl<A> ApplicationRuntime<A>
where
    A: App,
{
    fn handle_alert_event(
        &mut self,
        id: WindowId,
        event: &PlatformEvent,
        window: &dyn PlatformWindow,
    ) {
        let button = alert_button_bounds(window.viewport().logical_bounds());
        let Some(alert) = self
            .windows
            .get_mut(&id)
            .and_then(|state| state.alert.as_mut())
        else {
            return;
        };
        let mut dismiss = false;
        match event {
            PlatformEvent::KeyPressed { key, .. } if matches!(key, Key::Enter | Key::Escape) => {
                dismiss = true;
            }
            PlatformEvent::PointerMoved { x, y } => {
                let hovered = button.contains(Point::new(*x, *y));
                if alert.hovered != hovered {
                    alert.hovered = hovered;
                    window.request_redraw();
                }
            }
            PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            } => {
                alert.pressed = alert.hovered;
                window.request_redraw();
            }
            PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Released,
            } => {
                dismiss = alert.pressed && alert.hovered;
                alert.pressed = false;
                window.request_redraw();
            }
            PlatformEvent::PointerLeft => {
                alert.hovered = false;
                alert.pressed = false;
                window.request_redraw();
            }
            PlatformEvent::Focused(false) => window.activate(),
            _ => {}
        }
        if dismiss {
            self.remove_window(id);
            self.pending_window_commands
                .push(PlatformWindowCommand::Close { id });
        }
    }

    fn remove_window(&mut self, id: WindowId) {
        if let Some(mut state) = self.windows.remove(&id)
            && let Some(alert) = state.alert.as_mut()
            && let Some(on_dismiss) = alert.on_dismiss.take()
        {
            on_dismiss();
        }
        KEY_WINDOW.with(|key| {
            if key.get() == Some(id) {
                key.set(None);
            }
        });
        MAIN_WINDOW.with(|main| {
            if main.get() == Some(id) {
                main.set(self.windows.keys().next_back().copied());
            }
        });
    }
}

fn fallback_menu_bounds(request: &ContextMenuRequest, viewport: Rect, theme: &Theme) -> Rect {
    let inset = theme.spacing.extra_small;
    let height = request.items.iter().fold(inset * 2.0, |height, item| {
        height
            + if item.separator {
                theme.spacing.small
            } else {
                theme.layout.compact_control_height
            }
    });
    let width = theme.layout.popover_width.min(viewport.size.width);
    let preferred_x = request.position.x;
    let preferred_y = request.position.y;
    let x = preferred_x.clamp(
        viewport.origin.x,
        (viewport.origin.x + viewport.size.width - width).max(viewport.origin.x),
    );
    let below_y = preferred_y;
    let above_y = preferred_y - height;
    let y = if below_y + height <= viewport.origin.y + viewport.size.height {
        below_y
    } else {
        above_y.max(viewport.origin.y)
    };
    Rect::new(x, y, width, height.min(viewport.size.height))
}

fn alert_button_bounds(bounds: Rect) -> Rect {
    Rect::new(
        bounds.origin.x + bounds.size.width - 102.0,
        bounds.origin.y + bounds.size.height - 50.0,
        78.0,
        30.0,
    )
}

fn paint_alert_window(alert: &RuntimeAlert, bounds: Rect, context: &mut PaintContext<'_>) {
    Rectangle::new()
        .color(RectangleColor::Custom(context.theme.dialog.background))
        .paint(bounds, context);

    let mut dialog = AccessibilityNode::new(AccessibilityRole::Dialog, bounds);
    dialog.label = Some(alert.title.clone());
    dialog.value = Some(alert.message.clone());
    dialog.focus_scope = true;
    dialog.invalid = true;
    context.record_accessibility(dialog);

    let icon = Rect::new(bounds.origin.x + 24.0, bounds.origin.y + 28.0, 42.0, 42.0);
    Ellipse::new()
        .color(EllipseColor::Custom(context.theme.shell.alert))
        .paint(icon, context);
    Text::new("!")
        .accessibility_hidden(true)
        .font_size(26.0)
        .line_height(icon.size.height)
        .weight(750)
        .alignment(crate::typography::TextAlignment::Center)
        .color(context.theme.shell.inverse_text)
        .paint(icon, context);

    let text_x = icon.origin.x + icon.size.width + 18.0;
    let text_width = (bounds.size.width - text_x - 24.0).max(0.0);
    Text::styled(alert.title.clone(), crate::typography::TextRole::TitleSmall).paint(
        Rect::new(text_x, bounds.origin.y + 26.0, text_width, 26.0),
        context,
    );
    Text::body(alert.message.clone()).paint(
        Rect::new(text_x, bounds.origin.y + 60.0, text_width, 58.0),
        context,
    );

    let button = alert_button_bounds(bounds);
    let color = if alert.pressed {
        context.theme.shell.action_pressed
    } else if alert.hovered {
        context.theme.shell.action_hover
    } else {
        context.theme.shell.action
    };
    Rectangle::new()
        .color(RectangleColor::Custom(color))
        .radius(crate::theme::CornerRadius::Custom(7.0))
        .paint(button, context);
    Text::label("OK")
        .accessibility_hidden(true)
        .weight(650)
        .alignment(crate::typography::TextAlignment::Center)
        .color(context.theme.shell.inverse_text)
        .paint(button, context);

    let mut button_node = AccessibilityNode::new(AccessibilityRole::Button, button);
    button_node.label = Some(String::from("OK"));
    button_node.focusable = true;
    button_node.focused = true;
    context.record_accessibility(button_node);
}

fn fallback_menu_rows(
    request: &ContextMenuRequest,
    viewport: Rect,
    theme: &Theme,
) -> Vec<(usize, Rect)> {
    let menu = fallback_menu_bounds(request, viewport, theme);
    let inset = theme.spacing.extra_small;
    let mut y = menu.origin.y + inset;
    request
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let height = if item.separator {
                theme.spacing.small
            } else {
                theme.layout.compact_control_height
            };
            let row = Rect::new(
                menu.origin.x + inset,
                y,
                (menu.size.width - inset * 2.0).max(0.0),
                height,
            );
            y += height;
            (index, row)
        })
        .collect()
}

fn fallback_menu_command_at(
    request: &ContextMenuRequest,
    position: Point,
    viewport: Rect,
    theme: &Theme,
) -> Option<u32> {
    fallback_menu_rows(request, viewport, theme)
        .into_iter()
        .find_map(|(index, bounds)| {
            let item = request.items.get(index)?;
            (bounds.contains(position) && item.enabled && !item.separator)
                .then_some(item.command_id)
        })
}

fn paint_fallback_context_menu(
    menu: &FallbackContextMenu,
    viewport: Rect,
    context: &mut PaintContext<'_>,
) {
    let bounds = fallback_menu_bounds(&menu.request, viewport, context.theme);
    Rectangle::new()
        .color(RectangleColor::Custom(context.theme.card.background))
        .radius(context.theme.menu.surface_radius)
        .shadow(ShadowStyle::Card)
        .border(BorderStyle::custom(
            context.theme.card.border,
            context.theme.menu.surface_stroke_width,
        ))
        .paint(bounds, context);

    for (index, row) in fallback_menu_rows(&menu.request, viewport, context.theme) {
        let Some(item) = menu.request.items.get(index) else {
            continue;
        };
        if item.separator {
            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.colors.border))
                .paint(
                    Rect::new(
                        row.origin.x,
                        row.origin.y + row.size.height * 0.5,
                        row.size.width,
                        context.theme.divider.thickness,
                    ),
                    context,
                );
            continue;
        }

        let hovered = menu.pointer.is_some_and(|position| row.contains(position));
        let background = if item.checked {
            context.theme.colors.accent_soft
        } else if hovered && item.enabled {
            context.theme.menu.item_hovered_background
        } else {
            context.theme.menu.item_background
        };
        Rectangle::new()
            .color(RectangleColor::Custom(background))
            .radius(context.theme.menu.item_radius)
            .paint(row, context);
        let foreground = if item.enabled {
            context.theme.menu.foreground
        } else {
            context.theme.menu.disabled_foreground
        };
        Text::label(item.label.clone()).color(foreground).paint(
            Rect::new(
                row.origin.x + context.theme.menu.item_horizontal_padding,
                row.origin.y,
                (row.size.width - context.theme.menu.item_horizontal_padding * 2.0).max(0.0),
                row.size.height,
            ),
            context,
        );
    }
}

fn redraw_after_event(state_changed: bool, redraw_request: RedrawRequest) -> RedrawRequest {
    if state_changed {
        RedrawRequest::Full
    } else {
        redraw_request
    }
}

/// ViewKitアプリケーションを起動します.
///
/// アプリケーションの初期状態とウィンドウを作成し、
/// 現在のプラットフォームに対応するイベントループを開始します。
pub fn run<A>() -> Result<(), ViewKitError>
where
    A: App,
{
    let app = A::new();
    let options = app.window_for(WindowId::PRIMARY);

    let runtime = ApplicationRuntime::new(app);

    #[cfg(target_os = "linux")]
    {
        use crate::platform::linux::LinuxBackend;

        let backend = LinuxBackend::new(runtime, window_config(options));

        backend.run()?;

        Ok(())
    }

    #[cfg(target_os = "mochios")]
    {
        use crate::platform::mochios::MochiOsBackend;

        let backend = MochiOsBackend::new(runtime, window_config(options));

        backend.run()?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use crate::platform::windows::WindowsBackend;

        let backend = WindowsBackend::new(runtime, window_config(options));

        backend.run()?;

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "mochios", target_os = "windows")))]
    {
        let _ = runtime;
        let _ = options;

        Err(ViewKitError::UnsupportedPlatform)
    }
}

fn window_config(options: crate::app::WindowOptions) -> WindowConfig {
    WindowConfig {
        title: options.title().to_owned(),
        size: options.initial_size(),
        resizable: options.is_resizable(),
        fullscreen: options.is_fullscreen(),
        secure_overlay: options.is_secure_overlay(),
        system_modal: options.is_system_modal(),
    }
}

/// ViewKitアプリケーションの起動中に発生するエラーです。
#[derive(Debug, thiserror::Error)]
pub enum ViewKitError {
    #[cfg(target_os = "linux")]
    #[error(transparent)]
    Linux(#[from] crate::platform::linux::LinuxBackendError),

    #[cfg(target_os = "mochios")]
    #[error(transparent)]
    MochiOs(#[from] crate::platform::mochios::MochiOsBackendError),

    #[cfg(target_os = "windows")]
    #[error(transparent)]
    Windows(#[from] crate::platform::windows::WindowsBackendError),

    #[error("ViewKit does not support the current platform")]
    UnsupportedPlatform,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::WindowOptions;
    use crate::geometry::Size;
    use crate::platform::PlatformWindow;
    use crate::state::State;
    use crate::view::{Constraints, MeasureContext};
    use std::rc::Rc;

    #[test]
    fn state_change_expands_component_redraw_to_full_window() {
        let region = RedrawRequest::Region(Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(redraw_after_event(true, region), RedrawRequest::Full);
    }

    #[test]
    fn component_redraw_stays_regional_without_state_change() {
        let region = RedrawRequest::Region(Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(redraw_after_event(false, region), region);
    }

    #[test]
    fn application_exit_request_can_be_reset() {
        reset_exit_request();
        assert!(!exit_requested());
        request_exit();
        assert!(exit_requested());
        reset_exit_request();
        assert!(!exit_requested());
    }

    #[test]
    fn accepted_platform_close_is_window_scoped() {
        reset_exit_request();
        let app = PaintMutationApp {
            state: State::new(false),
            builds: Rc::new(Cell::new(0)),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let window = TestWindow(WindowId::PRIMARY);

        assert!(runtime.should_close_window(&window));

        assert!(!runtime.exit_requested());
        assert!(!runtime.windows.contains_key(&WindowId::PRIMARY));
        reset_exit_request();
    }

    #[test]
    fn focus_tracks_key_and_main_windows_independently() {
        reset_exit_request();
        let app = PaintMutationApp {
            state: State::new(false),
            builds: Rc::new(Cell::new(0)),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let window = TestWindow(WindowId::PRIMARY);

        assert_eq!(key_window(), None);
        assert_eq!(main_window(), Some(WindowId::PRIMARY));
        runtime.handle_event(PlatformEvent::Focused(true), &window);
        assert_eq!(key_window(), Some(WindowId::PRIMARY));
        assert_eq!(main_window(), Some(WindowId::PRIMARY));
        runtime.handle_event(PlatformEvent::Focused(false), &window);
        assert_eq!(key_window(), None);
        assert_eq!(main_window(), Some(WindowId::PRIMARY));
        reset_exit_request();
    }

    #[test]
    fn requested_close_waits_for_application_approval() {
        reset_exit_request();
        let app = PaintMutationApp {
            state: State::new(false),
            builds: Rc::new(Cell::new(0)),
        };
        let mut runtime = ApplicationRuntime::new(app);

        request_close_window(WindowId::PRIMARY);
        let commands = runtime.take_window_commands();
        assert!(matches!(
            commands.as_slice(),
            [PlatformWindowCommand::RequestClose { id }] if *id == WindowId::PRIMARY
        ));
        assert!(runtime.windows.contains_key(&WindowId::PRIMARY));

        close_window(WindowId::PRIMARY);
        let commands = runtime.take_window_commands();
        assert!(matches!(
            commands.as_slice(),
            [PlatformWindowCommand::Close { id }] if *id == WindowId::PRIMARY
        ));
        assert!(!runtime.windows.contains_key(&WindowId::PRIMARY));
        reset_exit_request();
    }

    #[test]
    fn external_activation_reopens_the_primary_window() {
        reset_exit_request();
        let app = PaintMutationApp {
            state: State::new(false),
            builds: Rc::new(Cell::new(0)),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let window = TestWindow(WindowId::PRIMARY);
        assert!(runtime.should_close_window(&window));
        assert!(runtime.windows.is_empty());

        runtime.reopen();
        let commands = runtime.take_window_commands();
        assert!(matches!(
            commands.as_slice(),
            [PlatformWindowCommand::Open { id, .. }] if *id == WindowId::PRIMARY
        ));
        assert!(runtime.windows.contains_key(&WindowId::PRIMARY));
        assert_eq!(main_window(), Some(WindowId::PRIMARY));
        reset_exit_request();
    }

    #[test]
    fn application_can_defer_a_platform_close_request() {
        reset_exit_request();
        let requested = Rc::new(Cell::new(false));
        let app = DeferredCloseApp {
            requested: Rc::clone(&requested),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let window = TestWindow(WindowId::PRIMARY);

        assert!(!runtime.should_close_window(&window));

        assert!(requested.get());
        assert!(!runtime.exit_requested());
        reset_exit_request();
    }

    #[test]
    fn new_windows_receive_independent_runtime_state() {
        reset_exit_request();
        let app = PaintMutationApp {
            state: State::new(false),
            builds: Rc::new(Cell::new(0)),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let second = request_new_window();

        let commands = runtime.take_window_commands();
        assert!(matches!(
            commands.as_slice(),
            [PlatformWindowCommand::Open { id, .. }] if *id == second
        ));

        let window = TestWindow(second);
        let viewport = window.viewport();
        runtime.handle_event(PlatformEvent::Resumed { viewport }, &window);

        assert!(runtime.windows.contains_key(&WindowId::PRIMARY));
        assert!(runtime.windows.contains_key(&second));
        assert!(runtime.windows[&WindowId::PRIMARY].root.is_none());
        assert!(runtime.windows[&second].root.is_some());
        reset_exit_request();
    }

    #[test]
    fn alert_windows_have_runtime_owned_content_and_dismissal() {
        reset_exit_request();
        let app = PaintMutationApp {
            state: State::new(false),
            builds: Rc::new(Cell::new(0)),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let dismissed = Rc::new(Cell::new(false));
        let dismissed_callback = Rc::clone(&dismissed);
        let alert = request_alert_window("Could not save", "The disk is full.", move || {
            dismissed_callback.set(true);
        });

        let commands = runtime.take_window_commands();
        assert!(matches!(
            commands.as_slice(),
            [PlatformWindowCommand::Open { id, config }]
                if *id == alert && !config.resizable && config.title == "Could not save"
        ));
        assert!(runtime.windows[&alert].alert.is_some());
        assert!(runtime.windows[&alert].root.is_none());

        assert!(runtime.should_close_window(&TestWindow(alert)));
        assert!(dismissed.get());
        assert!(!runtime.windows.contains_key(&alert));
        assert_eq!(main_window(), Some(WindowId::PRIMARY));
        reset_exit_request();
    }

    #[test]
    fn fallback_menu_clamps_to_the_window_and_ignores_disabled_items() {
        let request = ContextMenuRequest {
            request_id: 7,
            position: Point::new(90.0, 90.0),
            items: vec![
                crate::event::ContextMenuItem {
                    command_id: 1,
                    label: String::from("Enabled"),
                    enabled: true,
                    checked: false,
                    destructive: false,
                    separator: false,
                },
                crate::event::ContextMenuItem {
                    command_id: 2,
                    label: String::from("Disabled"),
                    enabled: false,
                    checked: false,
                    destructive: false,
                    separator: false,
                },
            ],
        };
        let viewport = Rect::new(0.0, 0.0, 200.0, 120.0);
        let bounds = fallback_menu_bounds(&request, viewport, &Theme::LIGHT);
        assert!(bounds.origin.x >= viewport.origin.x);
        assert!(bounds.origin.y >= viewport.origin.y);
        assert!(bounds.origin.x + bounds.size.width <= viewport.size.width);
        assert!(bounds.origin.y + bounds.size.height <= viewport.size.height);
        let rows = fallback_menu_rows(&request, viewport, &Theme::LIGHT);
        assert_eq!(
            fallback_menu_command_at(
                &request,
                Point::new(rows[0].1.origin.x + 1.0, rows[0].1.origin.y + 1.0),
                viewport,
                &Theme::LIGHT,
            ),
            Some(1)
        );
        assert_eq!(
            fallback_menu_command_at(
                &request,
                Point::new(rows[1].1.origin.x + 1.0, rows[1].1.origin.y + 1.0),
                viewport,
                &Theme::LIGHT,
            ),
            None
        );
    }

    #[test]
    fn state_changed_during_paint_rebuilds_before_the_next_draw() {
        let _ = take_state_changed();
        let builds = Rc::new(Cell::new(0));
        let state = State::new(false);
        let app = PaintMutationApp {
            state: state.clone(),
            builds: Rc::clone(&builds),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let mut display_list = DisplayList::default();

        let _ = runtime.draw(&TestWindow(WindowId::PRIMARY), &mut display_list);
        assert!(state.get());
        assert_eq!(builds.get(), 1);

        let _ = runtime.draw(&TestWindow(WindowId::PRIMARY), &mut display_list);
        assert_eq!(builds.get(), 2);
    }

    struct PaintMutationApp {
        state: State<bool>,
        builds: Rc<Cell<usize>>,
    }

    struct DeferredCloseApp {
        requested: Rc<Cell<bool>>,
    }

    struct TestWindow(WindowId);

    impl PlatformWindow for TestWindow {
        fn id(&self) -> WindowId {
            self.0
        }

        fn request_redraw(&self) {}

        fn set_title(&self, _title: &str) {}

        fn viewport(&self) -> Viewport {
            Viewport::new(Size::new(100.0, 100.0), 100, 100, 1.0)
        }
    }

    impl App for PaintMutationApp {
        type Body = PaintMutationView;

        fn new() -> Self {
            Self {
                state: State::new(false),
                builds: Rc::new(Cell::new(0)),
            }
        }

        fn window(&self) -> WindowOptions {
            WindowOptions::new("paint mutation test")
        }

        fn body(&self, _context: &ViewContext) -> Self::Body {
            self.builds.set(self.builds.get() + 1);
            PaintMutationView {
                state: self.state.clone(),
            }
        }
    }

    impl App for DeferredCloseApp {
        type Body = PaintMutationView;

        fn new() -> Self {
            Self {
                requested: Rc::new(Cell::new(false)),
            }
        }

        fn body(&self, _context: &ViewContext) -> Self::Body {
            PaintMutationView {
                state: State::new(true),
            }
        }

        fn close_requested(&mut self) -> bool {
            self.requested.set(true);
            false
        }
    }

    struct PaintMutationView {
        state: State<bool>,
    }

    impl View for PaintMutationView {
        fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
            constraints.constrain(Size::new(100.0, 100.0))
        }

        fn paint(&self, _bounds: Rect, context: &mut PaintContext<'_>) {
            if !self.state.get() {
                self.state.set(true);
                context.request_redraw_at(Instant::now());
            }
        }
    }
}
