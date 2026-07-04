use std::time::Instant;

use crate::app::{App, ViewContext};
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{EventContext, EventDispatcher};
use crate::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use crate::renderer::Viewport;
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use crate::view::{PaintContext, RedrawSchedule};

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
    pub(crate) fn new() -> Self {
        Self {
            app: A::new(),

            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),
        }
    }

    pub(crate) fn window_config(&self) -> WindowConfig {
        let options = self.app.window();

        WindowConfig {
            title: options.title,
            size: options.size,
            resizable: options.resizable,
        }
    }

    fn build_root(&self, viewport: Viewport) -> Box<dyn crate::view::View + 'static> {
        let context = ViewContext::new(viewport);

        self.app.body(&context)
    }
}

impl<A> PlatformApplication for ApplicationRuntime<A>
where
    A: App,
{
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
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
        let root = self.build_root(viewport);

        let mut context = EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

        self.event_dispatcher.dispatch(
            root.as_ref(),
            viewport.logical_bounds(),
            &event,
            &mut context,
        );

        if context.redraw_requested() {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();

        let root = self.build_root(viewport);

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
