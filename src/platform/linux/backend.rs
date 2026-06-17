use super::super::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use crate::draw_command::DisplayList;
use crate::geometry::Size;
use crate::platform::linux::SoftwareRenderer;
use crate::renderer::{Renderer, Viewport};
use softbuffer::Context;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::error::{EventLoopError, OsError};
use winit::event::WindowEvent;
use winit::event_loop::OwnedDisplayHandle;
use winit::event_loop::{
    ActiveEventLoop,
    ControlFlow,
    EventLoop,
};
use winit::window::{Window, WindowId};

#[derive(Debug, thiserror::Error)]
pub enum LinuxBackendError {
    #[error("イベントループの作成または実行に失敗しました: {0}")]
    EventLoop(#[from] EventLoopError),

    #[error("ウィンドウの作成に失敗しました: {0}")]
    Window(#[from] OsError),

    #[error("レンダラーの処理に失敗しました: {0}")]
    Renderer(
        #[from]
        crate::platform::linux::SoftwareRendererError,
    ),

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
}

pub struct LinuxBackend<A> {
    application: A,
    config: WindowConfig,

    context: Option<Context<OwnedDisplayHandle>>,
    window: Option<Rc<Window>>,
    renderer: Option<SoftwareRenderer>,

    runtime_error: Option<LinuxBackendError>,
}

impl<A> LinuxBackend<A>
where
    A: PlatformApplication,
{
    pub fn new(
        application: A,
        config: WindowConfig,
    ) -> Self {
        Self {
            application,
            config,

            context: None,
            window: None,
            renderer: None,

            runtime_error: None,
        }
    }
    pub fn run(mut self) -> Result<(), LinuxBackendError> {
        let event_loop = EventLoop::new()?;

        event_loop.set_control_flow(ControlFlow::Wait);

        self.context = Some(Context::new(
            event_loop.owned_display_handle(),
        )?);

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

        self.application
            .handle_event(event, &platform_window);
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
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
                self.runtime_error =
                    Some(LinuxBackendError::Window(error));

                event_loop.exit();
                return;
            }
        };

        let viewport = viewport_from_window(window.as_ref());

        let Some(context) = self.context.as_ref() else {
            event_loop.exit();
            return;
        };

        let renderer = match SoftwareRenderer::new(
            context,
            window.clone(),
            viewport,
        ) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.runtime_error =
                    Some(LinuxBackendError::Renderer(error));

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
        let Some(current_window_id) =
            self.window.as_ref().map(|window| window.id())
        else {
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

                let viewport =
                    viewport_from_physical(size, scale_factor);

                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.resize(viewport) {
                        self.runtime_error =
                            Some(LinuxBackendError::Renderer(error));

                        event_loop.exit();
                        return;
                    }
                }

                self.emit(PlatformEvent::Resized { viewport });
                self.request_redraw();
            }

            WindowEvent::ScaleFactorChanged {
                scale_factor,
                ..
            } => {
                let Some(size) =
                    self.window.as_ref().map(|window| window.inner_size())
                else {
                    return;
                };

                let viewport =
                    viewport_from_physical(size, scale_factor);

                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.resize(viewport) {
                        self.runtime_error =
                            Some(LinuxBackendError::Renderer(error));

                        event_loop.exit();
                        return;
                    }
                }

                self.emit(
                    PlatformEvent::ScaleFactorChanged {
                        viewport,
                    },
                );

                self.request_redraw();
            }

            WindowEvent::Focused(focused) => {
                self.emit(PlatformEvent::Focused(focused));
            }

            WindowEvent::RedrawRequested => {
                self.emit(PlatformEvent::RedrawRequested);

                let Some(window) = self.window.as_ref() else {
                    return;
                };

                let viewport = viewport_from_window(window);

                let mut display_list = DisplayList::new();

                self.application.draw(
                    viewport,
                    &mut display_list,
                );

                window.pre_present_notify();

                let result = self
                    .renderer
                    .as_mut()
                    .map(|renderer| renderer.render(&display_list));

                if let Some(Err(error)) = result {
                    self.runtime_error =
                        Some(LinuxBackendError::Renderer(error));

                    event_loop.exit();
                }
            }

            _ => {}
        }
    }
}

fn viewport_from_window(window: &Window) -> Viewport {
    viewport_from_physical(
        window.inner_size(),
        window.scale_factor(),
    )
}

fn viewport_from_physical(
    physical_size: PhysicalSize<u32>,
    scale_factor: f64,
) -> Viewport {
    let logical_size =
        physical_size.to_logical::<f64>(scale_factor);

    Viewport::new(
        Size::new(
            logical_size.width as f32,
            logical_size.height as f32,
        ),
        physical_size.width,
        physical_size.height,
        scale_factor,
    )
}