//! ViewKitアプリケーションとプラットフォームバックエンドをガッッッッタイ！します

use std::time::Instant;

use crate::app::{App, ViewContext};
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{EventContext, EventDispatcher};
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
        }
    }

    fn rebuild_root(&mut self, viewport: Viewport) {
        let context = ViewContext::new(viewport);

        self.root = Some(self.app.body(&context));
        self.viewport = Some(viewport);

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

        let redraw_requested = {
            let root = self
                .root
                .as_ref()
                .expect("root view must exist after ensure_root");

            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher
                .dispatch(root, viewport.logical_bounds(), &event, &mut context);

            context.redraw_requested()
        };

        let state_changed = take_state_changed();

        if state_changed {
            self.rebuild_root(viewport);
        }

        if redraw_requested || state_changed {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        self.ensure_root(viewport);

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

        root.paint(viewport.logical_bounds(), &mut context);
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
            },
        );

        backend.run()?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
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

    #[error("現在のプラットフォームはViewKitに対応していません")]
    UnsupportedPlatform,
}
