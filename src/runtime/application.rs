//! ViewKitアプリケーションとプラットフォームバックエンドをガッッッッタイ！します

use std::time::Instant;

use crate::app::{App, ViewContext};
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{EventContext, EventDispatcher, RedrawRequest};
use crate::geometry::Rect;
use crate::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use crate::renderer::Viewport;
use crate::state::take_state_changed;
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use crate::view::{PaintContext, RedrawSchedule, View};

/// `App`をプラットフォームバックエンド上で実行するランタイムです。
pub(crate) struct ApplicationRuntime<A>
where
    A: App,
{
    app: A,

    root: Option<A::Body>,
    viewport: Option<Viewport>,
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,

    event_dispatcher: EventDispatcher,
    redraw_schedule: RedrawSchedule,
    pending_redraw: RedrawRequest,
}

impl<A> ApplicationRuntime<A>
where
    A: App,
{
    pub(crate) fn new(app: A) -> Self {
        Self {
            app,

            root: None,
            viewport: None,
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),
            pending_redraw: RedrawRequest::None,
        }
    }

    fn rebuild_root(&mut self, viewport: Viewport) {
        self.rebuild_root_with_redraw(viewport, RedrawRequest::Full);
    }

    fn rebuild_root_with_redraw(&mut self, viewport: Viewport, redraw: RedrawRequest) {
        let context = ViewContext::new(viewport);

        self.root = Some(self.app.body(&context));
        self.viewport = Some(viewport);
        self.pending_redraw = redraw;

        let _ = take_state_changed();
    }

    fn ensure_root(&mut self, viewport: Viewport) {
        let viewport_changed = self.viewport != Some(viewport);

        if self.root.is_none() || viewport_changed {
            self.rebuild_root(viewport);
        }
    }
}

impl<A> PlatformApplication for ApplicationRuntime<A>
where
    A: App,
{
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
        match &event {
            PlatformEvent::Resumed { viewport }
            | PlatformEvent::Resized { viewport }
            | PlatformEvent::ScaleFactorChanged { viewport } => {
                self.rebuild_root(*viewport);
                return;
            }

            PlatformEvent::RedrawRequested | PlatformEvent::CloseRequested => {
                return;
            }

            _ => {}
        }

        let viewport = window.viewport();

        self.ensure_root(viewport);

        let (redraw_request, cursor_icon) = {
            let root = self
                .root
                .as_ref()
                .expect("root view must exist after ensure_root");

            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher
                .dispatch(root, viewport.logical_bounds(), &event, &mut context);

            (context.redraw_request(), context.cursor_icon())
        };

        if let Some(cursor_icon) = cursor_icon {
            window.set_cursor(cursor_icon);
        }

        let state_changed = take_state_changed();

        if state_changed {
            let redraw = if redraw_request.is_requested() {
                redraw_request
            } else {
                RedrawRequest::Full
            };
            self.rebuild_root_with_redraw(viewport, redraw);
        } else {
            self.pending_redraw = self.pending_redraw.merge(redraw_request);
        }

        if state_changed || redraw_request.is_requested() {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) -> Rect {
        self.ensure_root(viewport);

        let viewport_bounds = viewport.logical_bounds();

        let dirty_bounds = match std::mem::take(&mut self.pending_redraw) {
            RedrawRequest::Region(bounds) => bounds
                .intersection(viewport_bounds)
                .unwrap_or(viewport_bounds),

            RedrawRequest::None | RedrawRequest::Full => viewport_bounds,
        };

        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();

        let mut context = PaintContext::new(
            display_list,
            &self.theme,
            &self.typography,
            &mut self.text_measurer,
        )
        .with_redraw_schedule(&mut self.redraw_schedule);

        let root = self
            .root
            .as_ref()
            .expect("root view must exist after ensure_root");

        root.paint(viewport_bounds, &mut context);

        dirty_bounds
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        self.redraw_schedule.deadline()
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
    let options = app.window();

    let runtime = ApplicationRuntime::new(app);

    #[cfg(target_os = "linux")]
    {
        use crate::platform::linux::LinuxBackend;

        let backend = LinuxBackend::new(
            runtime,
            WindowConfig {
                title: options.title().to_owned(),
                size: options.initial_size(),
                resizable: options.is_resizable(),
                fullscreen: options.is_fullscreen(),
            },
        );

        backend.run()?;

        Ok(())
    }

    #[cfg(target_os = "mochios")]
    {
        use crate::platform::mochios::MochiOsBackend;

        let backend = MochiOsBackend::new(
            runtime,
            WindowConfig {
                title: options.title().to_owned(),
                size: options.initial_size(),
                resizable: options.is_resizable(),
                fullscreen: options.is_fullscreen(),
            },
        );

        backend.run()?;

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "mochios")))]
    {
        let _ = runtime;
        let _ = options;

        Err(ViewKitError::UnsupportedPlatform)
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

    #[error("現在のプラットフォームはViewKitに対応していません")]
    UnsupportedPlatform,
}
