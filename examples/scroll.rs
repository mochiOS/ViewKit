use viewkit::components::{
    Background,
    Divider,
    Rectangle,
    RectangleColor,
    Scroll,
    ScrollAxis,
    ScrollState,
    VStack,
};
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::event::{
    EventContext,
    EventDispatcher,
};
use viewkit::geometry::Size;
use viewkit::layout::{
    StackAlignment,
    StackDistribution,
    StackGap,
    ViewExt,
};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use viewkit::renderer::Viewport;
use viewkit::theme::Theme;
use viewkit::typography::{
    TextMeasurer,
    Typography,
};
use viewkit::view::{
    PaintContext,
    View,
};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,

    event_dispatcher:
        EventDispatcher,

    scroll_state:
        ScrollState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme:
            Theme::DEFAULT,

            typography:
            Typography::DEFAULT,

            text_measurer:
            TextMeasurer::new(),

            event_dispatcher:
            EventDispatcher::new(),

            scroll_state:
            ScrollState::new(),
        }
    }

    fn build_root(
        &self,
    ) -> VStack {
        let scroll_content =
            VStack::new()
                .gap(
                    StackGap::Large,
                )
                .alignment(
                    StackAlignment::Center,
                )
                .distribution(
                    StackDistribution::Start,
                )
                .child(
                    Rectangle::new()
                        .color(
                            RectangleColor::Accent,
                        )
                        .frame(
                            320.0,
                            120.0,
                        ),
                )
                .child(
                    Divider::new(),
                )
                .child(
                    Rectangle::new()
                        .color(
                            RectangleColor::Destructive,
                        )
                        .frame(
                            320.0,
                            120.0,
                        ),
                )
                .child(
                    Divider::new(),
                )
                .child(
                    Rectangle::new()
                        .color(
                            RectangleColor::Accent,
                        )
                        .frame(
                            320.0,
                            120.0,
                        ),
                )
                .child(
                    Divider::new(),
                )
                .child(
                    Rectangle::new()
                        .color(
                            RectangleColor::Destructive,
                        )
                        .frame(
                            320.0,
                            120.0,
                        ),
                )
                .child(
                    Divider::new(),
                )
                .child(
                    Rectangle::new()
                        .color(
                            RectangleColor::Accent,
                        )
                        .frame(
                            320.0,
                            120.0,
                        ),
                )
                .child(
                    Divider::new(),
                )
                .child(
                    Rectangle::new()
                        .color(
                            RectangleColor::Destructive,
                        )
                        .frame(
                            320.0,
                            120.0,
                        ),
                );

        let scroll =
            Scroll::new(
                self.scroll_state
                    .clone(),
            )
                .axis(
                    ScrollAxis::Vertical,
                )
                .content(
                    scroll_content.frame(
                        420.0,
                        1040.0,
                    ),
                );

        let card =
            Background::new()
                .background(
                    Rectangle::new()
                        .color(
                            RectangleColor::
                            ElevatedSurface,
                        ),
                )
                .content(
                    scroll,
                );

        VStack::new()
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                card.frame(
                    420.0,
                    320.0,
                ),
            )
    }
}

impl PlatformApplication
for ExampleApplication
{
    fn handle_event(
        &mut self,
        event: PlatformEvent,
        window: &dyn PlatformWindow,
    ) {
        let root =
            self.build_root();

        let redraw_requested = {
            let mut context =
                EventContext::new(
                    &self.theme,
                    &self.typography,
                    &mut self.text_measurer,
                );

            self.event_dispatcher.dispatch(
                &root,
                window
                    .viewport()
                    .logical_bounds(),
                &event,
                &mut context,
            );

            context.redraw_requested()
        };

        if redraw_requested {
            window.request_redraw();
        }

        match event {
            PlatformEvent::Resumed {
                viewport,
            } => {
                println!(
                    "resumed: {viewport:?}"
                );
            }

            PlatformEvent::Resized {
                viewport,
            } => {
                println!(
                    "resized: {viewport:?}"
                );
            }

            PlatformEvent::ScaleFactorChanged {
                viewport,
            } => {
                println!(
                    "scale factor changed: \
                     {viewport:?}"
                );
            }

            PlatformEvent::Focused(
                focused,
            ) => {
                println!(
                    "focused: {focused}"
                );
            }

            PlatformEvent::CloseRequested => {
                println!(
                    "close requested"
                );
            }

            PlatformEvent::PointerMoved {
                ..
            }
            | PlatformEvent::PointerButton {
                ..
            }
            | PlatformEvent::PointerLeft
            | PlatformEvent::Scroll {
                ..
            }
            | PlatformEvent::RedrawRequested => {}
        }
    }

    fn draw(
        &mut self,
        viewport: Viewport,
        display_list: &mut DisplayList,
    ) {
        display_list.push(
            DrawCommand::Clear {
                color: self
                    .theme
                    .colors
                    .background,
            },
        );

        let root =
            self.build_root();

        let mut context =
            PaintContext {
                display_list,

                theme:
                &self.theme,

                typography:
                &self.typography,

                text_measurer:
                &mut self.text_measurer,
            };

        root.paint(
            viewport.logical_bounds(),
            &mut context,
        );
    }
}

fn main(
) -> Result<
    (),
    Box<dyn std::error::Error>,
> {
    let application =
        ExampleApplication::new();

    let backend =
        LinuxBackend::new(
            application,

            WindowConfig {
                title: String::from(
                    "ViewKit Scroll Example",
                ),

                size: Size::new(
                    720.0,
                    520.0,
                ),

                resizable: true,
            },
        );

    backend.run()?;

    Ok(())
}