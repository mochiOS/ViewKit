//! winitを使用したLinux/Waylandバックエンド

use super::super::{
    ButtonState, Key, KeyModifiers, PlatformApplication, PlatformEvent, PlatformWindow,
    PlatformWindowCommand, PointerButton, WindowConfig,
};

use super::GpuRenderer;
use crate::app::WindowId as ViewKitWindowId;
use crate::draw_command::DisplayList;
use crate::geometry::Size;
use crate::renderer::{Renderer, Viewport};

use crate::platform::CursorIcon;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::CursorIcon as WinitCursorIcon;
use winit::window::{Fullscreen, Window, WindowId};

const LINE_SCROLL_PIXELS: f32 = 40.0;

const BACK_MOUSE_BUTTON_ID: u16 = 4;
const FORWARD_MOUSE_BUTTON_ID: u16 = 5;

#[derive(Debug, thiserror::Error)]
pub enum LinuxBackendError {
    #[error("Failed to create or run the event loop: {0}")]
    EventLoop(#[from] EventLoopError),

    #[error("Failed to create the window: {0}")]
    Window(#[from] OsError),

    #[error("The GPU renderer failed: {0}")]
    GpuRenderer(#[from] super::GpuRendererError),
}

struct WinitWindow<'a> {
    id: ViewKitWindowId,
    inner: &'a Window,
}

impl PlatformWindow for WinitWindow<'_> {
    fn id(&self) -> ViewKitWindowId {
        self.id
    }

    fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    fn viewport(&self) -> Viewport {
        viewport_from_window(self.inner)
    }

    fn set_cursor(&self, cursor: CursorIcon) {
        let cursor = match cursor {
            CursorIcon::Default => WinitCursorIcon::Default,

            CursorIcon::Pointer => WinitCursorIcon::Pointer,

            CursorIcon::Text => WinitCursorIcon::Text,

            CursorIcon::EwResize => WinitCursorIcon::EwResize,

            CursorIcon::NsResize => WinitCursorIcon::NsResize,

            CursorIcon::NwseResize => WinitCursorIcon::NwseResize,

            CursorIcon::NeswResize => WinitCursorIcon::NeswResize,
        };

        self.inner.set_cursor(cursor);
    }
}

pub struct LinuxBackend<A> {
    application: A,
    config: WindowConfig,
    windows: HashMap<WindowId, LinuxWindowState>,
    ids: HashMap<ViewKitWindowId, WindowId>,

    runtime_error: Option<LinuxBackendError>,
}

struct LinuxWindowState {
    id: ViewKitWindowId,
    window: Rc<Window>,
    renderer: GpuRenderer,
    modifiers: KeyModifiers,
    pending_pointer_move: Option<(f32, f32)>,
}

impl<A> LinuxBackend<A>
where
    A: PlatformApplication,
{
    pub fn new(application: A, config: WindowConfig) -> Self {
        Self {
            application,
            config,
            windows: HashMap::new(),
            ids: HashMap::new(),
            runtime_error: None,
        }
    }

    pub fn run(mut self) -> Result<(), LinuxBackendError> {
        let event_loop = EventLoop::new()?;

        event_loop.set_control_flow(ControlFlow::Wait);

        let result = event_loop.run_app(&mut self);

        if let Some(error) = self.runtime_error.take() {
            return Err(error);
        }

        result?;

        Ok(())
    }

    fn emit(&mut self, window_id: WindowId, event: PlatformEvent) {
        let Some(state) = self.windows.get(&window_id) else {
            return;
        };
        let window = Rc::clone(&state.window);
        let platform_window = WinitWindow {
            id: state.id,
            inner: window.as_ref(),
        };
        self.application.handle_event(event, &platform_window);
    }

    fn request_redraw(&self, id: ViewKitWindowId) {
        if let Some(window_id) = self.ids.get(&id)
            && let Some(state) = self.windows.get(window_id)
        {
            state.window.request_redraw();
        }
    }

    fn resize_renderer(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        viewport: Viewport,
    ) -> bool {
        if let Some(state) = self.windows.get_mut(&id)
            && let Err(error) = state.renderer.resize(viewport)
        {
            self.runtime_error = Some(error.into());
            event_loop.exit();
            return false;
        }
        true
    }

    fn render(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId) {
        let Some(state) = self.windows.get(&window_id) else {
            return;
        };
        let id = state.id;
        let window = Rc::clone(&state.window);
        let mut display_list = DisplayList::new();
        let platform_window = WinitWindow {
            id,
            inner: window.as_ref(),
        };
        let dirty_bounds = self.application.draw(&platform_window, &mut display_list);
        window.pre_present_notify();
        if let Some(state) = self.windows.get_mut(&window_id)
            && let Err(error) = state.renderer.render(&display_list, dirty_bounds)
        {
            self.runtime_error = Some(error.into());
            event_loop.exit();
        }
    }

    fn flush_pending_pointer_move(&mut self, window_id: WindowId) {
        let Some((x, y)) = self
            .windows
            .get_mut(&window_id)
            .and_then(|state| state.pending_pointer_move.take())
        else {
            return;
        };
        self.emit(window_id, PlatformEvent::PointerMoved { x, y });
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: ViewKitWindowId,
        config: WindowConfig,
    ) -> Result<(), LinuxBackendError> {
        if self.ids.contains_key(&id) {
            return Ok(());
        }
        let attributes = Window::default_attributes()
            .with_title(config.title)
            .with_inner_size(LogicalSize::new(
                config.size.width as f64,
                config.size.height as f64,
            ))
            .with_resizable(config.resizable)
            .with_fullscreen(config.fullscreen.then_some(Fullscreen::Borderless(None)));
        let window = Rc::new(event_loop.create_window(attributes)?);
        let viewport = viewport_from_window(window.as_ref());
        let renderer =
            GpuRenderer::new(window.clone(), viewport, event_loop.owned_display_handle())?;
        let window_id = window.id();
        self.ids.insert(id, window_id);
        self.windows.insert(
            window_id,
            LinuxWindowState {
                id,
                window,
                renderer,
                modifiers: KeyModifiers::default(),
                pending_pointer_move: None,
            },
        );
        self.emit(window_id, PlatformEvent::Resumed { viewport });
        self.request_redraw(id);
        Ok(())
    }

    fn process_window_commands(&mut self, event_loop: &ActiveEventLoop) {
        for command in self.application.take_window_commands() {
            match command {
                PlatformWindowCommand::Open { id, config } => {
                    if let Err(error) = self.create_window(event_loop, id, config) {
                        self.runtime_error = Some(error);
                        event_loop.exit();
                        return;
                    }
                }
                PlatformWindowCommand::Close { id } => {
                    if let Some(window_id) = self.ids.remove(&id) {
                        self.windows.remove(&window_id);
                    }
                }
                PlatformWindowCommand::RequestClose { id } => {
                    if let Some(window_id) = self.ids.get(&id).copied()
                        && let Some(state) = self.windows.get(&window_id)
                    {
                        let window = Rc::clone(&state.window);
                        let platform_window = WinitWindow {
                            id,
                            inner: window.as_ref(),
                        };
                        if self.application.should_close_window(&platform_window) {
                            self.windows.remove(&window_id);
                            self.ids.remove(&id);
                        }
                    }
                }
                PlatformWindowCommand::Redraw { id } => self.request_redraw(id),
            }
        }
    }
}

impl<A> ApplicationHandler for LinuxBackend<A>
where
    A: PlatformApplication,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() || self.runtime_error.is_some() {
            return;
        }
        if let Err(error) =
            self.create_window(event_loop, ViewKitWindowId::PRIMARY, self.config.clone())
        {
            self.runtime_error = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.windows.get(&window_id) else {
            return;
        };
        let id = state.id;
        let window = Rc::clone(&state.window);

        match event {
            WindowEvent::CloseRequested => {
                let platform_window = WinitWindow {
                    id,
                    inner: window.as_ref(),
                };
                if self.application.should_close_window(&platform_window) {
                    self.windows.remove(&window_id);
                    self.ids.remove(&id);
                    if self.windows.is_empty() {
                        event_loop.exit();
                    }
                }
            }

            WindowEvent::Resized(size) => {
                let scale_factor = window.scale_factor();
                let viewport = viewport_from_physical(size, scale_factor);
                if !self.resize_renderer(event_loop, window_id, viewport) {
                    return;
                }
                self.emit(window_id, PlatformEvent::Resized { viewport });
                self.request_redraw(id);
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = window.inner_size();
                let viewport = viewport_from_physical(size, scale_factor);
                if !self.resize_renderer(event_loop, window_id, viewport) {
                    return;
                }
                self.emit(window_id, PlatformEvent::ScaleFactorChanged { viewport });
                self.request_redraw(id);
            }

            WindowEvent::Focused(focused) => {
                if !focused {
                    if let Some(state) = self.windows.get_mut(&window_id) {
                        state.modifiers = KeyModifiers::default();
                    }
                }
                self.emit(window_id, PlatformEvent::Focused(focused));
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.pending_pointer_move = Some(physical_position_to_logical(
                        position,
                        window.scale_factor(),
                    ));
                }
            }

            WindowEvent::CursorLeft { .. } => {
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.pending_pointer_move = None;
                }
                self.emit(window_id, PlatformEvent::PointerLeft);
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();

                let mut bits = 0;
                if state.shift_key() {
                    bits |= KeyModifiers::SHIFT;
                }
                if state.control_key() {
                    bits |= KeyModifiers::CONTROL;
                }
                if state.alt_key() {
                    bits |= KeyModifiers::ALT;
                }
                if state.super_key() {
                    bits |= KeyModifiers::SUPER;
                }
                if let Some(state) = self.windows.get_mut(&window_id) {
                    state.modifiers = KeyModifiers::from_bits(bits);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.flush_pending_pointer_move(window_id);
                self.emit(
                    window_id,
                    PlatformEvent::PointerButton {
                        button: convert_mouse_button(button),
                        state: convert_button_state(state),
                    },
                );
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.flush_pending_pointer_move(window_id);
                let (delta_x, delta_y) = scroll_delta_to_logical(delta, window.scale_factor());
                self.emit(window_id, PlatformEvent::Scroll { delta_x, delta_y });
            }

            WindowEvent::HoveredFile(path) => {
                self.flush_pending_pointer_move(window_id);
                self.emit(window_id, PlatformEvent::FileHovered { path });
            }

            WindowEvent::HoveredFileCancelled => {
                self.emit(window_id, PlatformEvent::FileHoverCancelled);
            }

            WindowEvent::DroppedFile(path) => {
                self.flush_pending_pointer_move(window_id);
                self.emit(window_id, PlatformEvent::FileDropped { path });
            }

            WindowEvent::RedrawRequested => {
                self.emit(window_id, PlatformEvent::RedrawRequested);
                self.render(event_loop, window_id);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                let modifiers = self
                    .windows
                    .get(&window_id)
                    .map_or(KeyModifiers::default(), |state| state.modifiers);
                if let Some(key) = convert_key(&event.logical_key) {
                    self.emit(window_id, PlatformEvent::KeyPressed { key, modifiers });
                }

                let platform_event = match &event.logical_key {
                    WinitKey::Character(character)
                        if modifiers.shortcut() && character.as_str().eq_ignore_ascii_case("a") =>
                    {
                        Some(PlatformEvent::SelectAll)
                    }
                    WinitKey::Named(NamedKey::Backspace) => Some(PlatformEvent::Backspace),
                    WinitKey::Named(NamedKey::Delete) => Some(PlatformEvent::Delete),
                    WinitKey::Named(NamedKey::ArrowLeft) => Some(if modifiers.shift() {
                        PlatformEvent::SelectLeft
                    } else {
                        PlatformEvent::ArrowLeft
                    }),
                    WinitKey::Named(NamedKey::ArrowRight) => Some(if modifiers.shift() {
                        PlatformEvent::SelectRight
                    } else {
                        PlatformEvent::ArrowRight
                    }),
                    WinitKey::Named(NamedKey::Home) => Some(if modifiers.shift() {
                        PlatformEvent::SelectHome
                    } else {
                        PlatformEvent::Home
                    }),
                    WinitKey::Named(NamedKey::End) => Some(if modifiers.shift() {
                        PlatformEvent::SelectEnd
                    } else {
                        PlatformEvent::End
                    }),

                    _ => None,
                };

                if let Some(platform_event) = platform_event {
                    self.emit(window_id, platform_event);
                    return;
                }

                if modifiers.shortcut() || modifiers.alt() {
                    return;
                }

                let Some(text) = event.text else {
                    return;
                };

                let text: String = text
                    .chars()
                    .filter(|character| !character.is_control())
                    .collect();

                if text.is_empty() {
                    return;
                }

                self.emit(window_id, PlatformEvent::TextInput { text });
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window_ids = self.windows.keys().copied().collect::<Vec<_>>();
        for window_id in window_ids {
            self.flush_pending_pointer_move(window_id);
        }
        self.process_window_commands(event_loop);
        if self.application.exit_requested() {
            event_loop.exit();
            return;
        }
        let next = self
            .ids
            .keys()
            .filter_map(|id| {
                self.application
                    .next_redraw_at(*id)
                    .map(|deadline| (*id, deadline))
            })
            .min_by_key(|(_, deadline)| *deadline);
        let Some((id, deadline)) = next else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };

        if deadline <= Instant::now() {
            self.request_redraw(id);
            event_loop.set_control_flow(ControlFlow::Wait);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}

fn viewport_from_window(window: &Window) -> Viewport {
    viewport_from_physical(window.inner_size(), window.scale_factor())
}

fn convert_key(key: &WinitKey) -> Option<Key> {
    match key {
        WinitKey::Named(named) => Some(match named {
            NamedKey::Escape => Key::Escape,
            NamedKey::Tab => Key::Tab,
            NamedKey::Enter => Key::Enter,
            NamedKey::Space => Key::Space,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::ArrowUp => Key::ArrowUp,
            NamedKey::ArrowDown => Key::ArrowDown,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            _ => return None,
        }),
        WinitKey::Character(text) => {
            let mut characters = text.chars();
            let character = characters.next()?;
            characters
                .next()
                .is_none()
                .then_some(Key::Character(character))
        }
        _ => None,
    }
}

fn viewport_from_physical(physical_size: PhysicalSize<u32>, scale_factor: f64) -> Viewport {
    let scale_factor = valid_scale_factor(scale_factor);

    let logical_size = physical_size.to_logical::<f64>(scale_factor);

    Viewport::new(
        Size::new(logical_size.width as f32, logical_size.height as f32),
        physical_size.width,
        physical_size.height,
        scale_factor,
    )
}

fn physical_position_to_logical(position: PhysicalPosition<f64>, scale_factor: f64) -> (f32, f32) {
    let scale_factor = valid_scale_factor(scale_factor);

    let logical_position = position.to_logical::<f64>(scale_factor);

    (logical_position.x as f32, logical_position.y as f32)
}

fn scroll_delta_to_logical(delta: MouseScrollDelta, scale_factor: f64) -> (f32, f32) {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * LINE_SCROLL_PIXELS, y * LINE_SCROLL_PIXELS),

        MouseScrollDelta::PixelDelta(position) => {
            let scale_factor = valid_scale_factor(scale_factor) as f32;

            (
                position.x as f32 / scale_factor,
                position.y as f32 / scale_factor,
            )
        }
    }
}

fn convert_mouse_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,

        MouseButton::Right => PointerButton::Secondary,

        MouseButton::Middle => PointerButton::Middle,

        MouseButton::Back => PointerButton::Other(BACK_MOUSE_BUTTON_ID),

        MouseButton::Forward => PointerButton::Other(FORWARD_MOUSE_BUTTON_ID),

        MouseButton::Other(button) => PointerButton::Other(button),
    }
}

fn convert_button_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,

        ElementState::Released => ButtonState::Released,
    }
}

fn valid_scale_factor(scale_factor: f64) -> f64 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}
