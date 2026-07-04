//! ViewKitアプリケーションとプラットフォームバックエンドをガッッッッタイ！します

use std::time::Instant;

use crate::app::{App, ViewContext};
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{EventContext, EventDispatcher};
use crate::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use crate::renderer::Viewport;
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use crate::view::{PaintContext, RedrawSchedule, View};

/// `App`をプラットフォームバックエンド上で実行するランタイムです。
pub(crate) struct ApplicationRuntime<A>
where
    A: App,
{
    app: A,

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

            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),
        }
    }
}

impl<A> PlatformApplication for ApplicationRuntime<A>
where
    A: App,
{
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
        /*
         * これらのイベントはバックエンド側で処理され、
         * Viewへ配送する必要がありません。
         */
        if matches!(
            event,
            PlatformEvent::Resumed { .. }
                | PlatformEvent::Resized { .. }
                | PlatformEvent::ScaleFactorChanged { .. }
                | PlatformEvent::RedrawRequested
                | PlatformEvent::CloseRequested
        ) {
            return;
        }

        let viewport = window.viewport();

        let root = {
            let context = ViewContext::new(viewport);

            self.app.body(&context)
        };

        let redraw_requested = {
            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher
                .dispatch(&root, viewport.logical_bounds(), &event, &mut context);

            context.redraw_requested()
        };

        if redraw_requested {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();

        let root = {
            let context = ViewContext::new(viewport);

            self.app.body(&context)
        };

        let mut context = PaintContext::new(
            display_list,
            &self.theme,
            &self.typography,
            &mut self.text_measurer,
        )
        .with_redraw_schedule(&mut self.redraw_schedule);

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
