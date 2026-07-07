//! winitを使用したLinux/Waylandバックエンド

use super::super::{
    ButtonState, PlatformApplication, PlatformEvent, PlatformWindow, PointerButton, WindowConfig,
};

use crate::draw_command::DisplayList;
use crate::geometry::Size;
use crate::platform::linux::SoftwareRenderer;
use crate::renderer::{Renderer, Viewport};

use softbuffer::Context;

use crate::platform::CursorIcon;
use std::rc::Rc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{Key, NamedKey};
use winit::window::CursorIcon as WinitCursorIcon;
use winit::window::{Window, WindowId};

const LINE_SCROLL_PIXELS: f32 = 40.0;

const BACK_MOUSE_BUTTON_ID: u16 = 4;
const FORWARD_MOUSE_BUTTON_ID: u16 = 5;

#[derive(Debug, thiserror::Error)]
pub enum LinuxBackendError {
    #[error("イベントループの作成または実行に失敗しました: {0}")]
    EventLoop(#[from] EventLoopError),

    #[error("ウィンドウの作成に失敗しました: {0}")]
    Window(#[from] OsError),

    #[error("レンダラーの処理に失敗しました: {0}")]
    Renderer(#[from] crate::platform::linux::SoftwareRendererError),

    #[error("softbufferの初期化に失敗しました: {0}")]
    SoftBuffer(#[from] softbuffer::SoftBufferError),
}

struct WinitWindow<'a> {
    inner: &'a Window,
}

impl PlatformWindow for WinitWindow<'_> {
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

    context: Option<Context<OwnedDisplayHandle>>,
    window: Option<Rc<Window>>,
    renderer: Option<SoftwareRenderer>,

    shift_pressed: bool,
    shortcut_pressed: bool,

    runtime_error: Option<LinuxBackendError>,
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

            context: None,
            window: None,
            renderer: None,

            shift_pressed: false,
            shortcut_pressed: false,

            runtime_error: None,
            pending_pointer_move: None,
        }
    }

    pub fn run(mut self) -> Result<(), LinuxBackendError> {
        let event_loop = EventLoop::new()?;

        event_loop.set_control_flow(ControlFlow::Wait);

        self.context = Some(Context::new(event_loop.owned_display_handle())?);

        let result = event_loop.run_app(&mut self);

        if let Some(error) = self.runtime_error.take() {
            return Err(error);
        }

        result?;

        Ok(())
    }

    fn emit(&mut self, event: PlatformEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let platform_window = WinitWindow {
            inner: window.as_ref(),
        };

        self.application.handle_event(event, &platform_window);
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn resize_renderer(&mut self, event_loop: &ActiveEventLoop, viewport: Viewport) -> bool {
        let Some(renderer) = self.renderer.as_mut() else {
            return true;
        };

        if let Err(error) = renderer.resize(viewport) {
            self.runtime_error = Some(LinuxBackendError::Renderer(error));

            event_loop.exit();

            return false;
        }

        true
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let viewport = viewport_from_window(window.as_ref());

        let mut display_list = DisplayList::new();

        let dirty_bounds = self.application.draw(viewport, &mut display_list);

        window.pre_present_notify();

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        if let Err(error) = renderer.render(&display_list, dirty_bounds) {
            self.runtime_error = Some(LinuxBackendError::Renderer(error));

            event_loop.exit();
        }
    }

    fn flush_pending_pointer_move(&mut self) {
        let Some((x, y)) = self.pending_pointer_move.take() else {
            return;
        };

        self.emit(PlatformEvent::PointerMoved { x, y });
    }
}

impl<A> ApplicationHandler for LinuxBackend<A>
where
    A: PlatformApplication,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.runtime_error.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(
                self.config.size.width as f64,
                self.config.size.height as f64,
            ))
            .with_resizable(self.config.resizable);

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Rc::new(window),

            Err(error) => {
                self.runtime_error = Some(LinuxBackendError::Window(error));

                event_loop.exit();

                return;
            }
        };

        let viewport = viewport_from_window(window.as_ref());

        let Some(context) = self.context.as_ref() else {
            event_loop.exit();

            return;
        };

        let renderer = match SoftwareRenderer::new(context, window.clone(), viewport) {
            Ok(renderer) => renderer,

            Err(error) => {
                self.runtime_error = Some(LinuxBackendError::Renderer(error));

                event_loop.exit();

                return;
            }
        };

        self.window = Some(window);

        self.renderer = Some(renderer);

        self.emit(PlatformEvent::Resumed { viewport });

        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(current_window_id) = self.window.as_ref().map(|window| window.id()) else {
            return;
        };

        if current_window_id != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.emit(PlatformEvent::CloseRequested);

                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);

                let viewport = viewport_from_physical(size, scale_factor);

                if !self.resize_renderer(event_loop, viewport) {
                    return;
                }

                self.emit(PlatformEvent::Resized { viewport });

                self.request_redraw();
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let Some(size) = self.window.as_ref().map(|window| window.inner_size()) else {
                    return;
                };

                let viewport = viewport_from_physical(size, scale_factor);

                if !self.resize_renderer(event_loop, viewport) {
                    return;
                }

                self.emit(PlatformEvent::ScaleFactorChanged { viewport });

                self.request_redraw();
            }

            WindowEvent::Focused(focused) => {
                if !focused {
                    self.shift_pressed = false;
                    self.shortcut_pressed = false;
                }

                self.emit(PlatformEvent::Focused(focused));
            }

            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);

                self.pending_pointer_move =
                    Some(physical_position_to_logical(position, scale_factor));
            }

            WindowEvent::CursorLeft { .. } => {
                self.pending_pointer_move = None;
                self.emit(PlatformEvent::PointerLeft);
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();

                self.shift_pressed = state.shift_key();

                self.shortcut_pressed = state.control_key() || state.super_key();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.flush_pending_pointer_move();
                self.emit(PlatformEvent::PointerButton {
                    button: convert_mouse_button(button),
                    state: convert_button_state(state),
                });
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.flush_pending_pointer_move();
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);

                let (delta_x, delta_y) = scroll_delta_to_logical(delta, scale_factor);

                self.emit(PlatformEvent::Scroll { delta_x, delta_y });
            }

            WindowEvent::RedrawRequested => {
                self.emit(PlatformEvent::RedrawRequested);

                self.render(event_loop);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                let platform_event = match &event.logical_key {
                    Key::Character(character)
                        if self.shortcut_pressed
                            && character.as_str().eq_ignore_ascii_case("a") =>
                    {
                        Some(PlatformEvent::SelectAll)
                    }
                    Key::Named(NamedKey::Backspace) => Some(PlatformEvent::Backspace),
                    Key::Named(NamedKey::Delete) => Some(PlatformEvent::Delete),
                    Key::Named(NamedKey::ArrowLeft) => Some(if self.shift_pressed {
                        PlatformEvent::SelectLeft
                    } else {
                        PlatformEvent::ArrowLeft
                    }),
                    Key::Named(NamedKey::ArrowRight) => Some(if self.shift_pressed {
                        PlatformEvent::SelectRight
                    } else {
                        PlatformEvent::ArrowRight
                    }),
                    Key::Named(NamedKey::Home) => Some(if self.shift_pressed {
                        PlatformEvent::SelectHome
                    } else {
                        PlatformEvent::Home
                    }),
                    Key::Named(NamedKey::End) => Some(if self.shift_pressed {
                        PlatformEvent::SelectEnd
                    } else {
                        PlatformEvent::End
                    }),

                    _ => None,
                };

                if let Some(platform_event) = platform_event {
                    self.emit(platform_event);
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

                self.emit(PlatformEvent::TextInput { text });
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.flush_pending_pointer_move();
        let Some(deadline) = self.application.next_redraw_at() else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };

        if deadline <= Instant::now() {
            self.request_redraw();
            event_loop.set_control_flow(ControlFlow::Wait);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}

fn viewport_from_window(window: &Window) -> Viewport {
    viewport_from_physical(window.inner_size(), window.scale_factor())
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
