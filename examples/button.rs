use viewkit::components::{Button, ButtonColor, ButtonInteractionState, Text, VStack};
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
use viewkit::theme::{Color, Theme};
use viewkit::typography::Typography;
use viewkit::view::{
    PaintContext,
    View,
};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,

    event_dispatcher: EventDispatcher,

    primary_button_state:
        ButtonInteractionState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,

            event_dispatcher:
            EventDispatcher::new(),

            primary_button_state:
            ButtonInteractionState::new(),
        }
    }

    fn build_root(&self) -> VStack {
        let primary_button =
            Button::new(
                self.primary_button_state
                    .clone(),
            )
                .color(
                    ButtonColor::Accent,
                )
                .content(
                    Text::new(
                        "プライマリーボタン",
                    )
                        .font_size(
                            16.0,
                        )
                        .line_height(
                            24.0,
                        )
                        .weight(
                            600,
                        )
                        .color(
                            Color::WHITE,
                        )
                        .frame(
                            80.0,
                            24.0,
                        ),
                );

        VStack::new()
            .gap(
                StackGap::Large,
            )
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                primary_button.frame(
                    240.0,
                    60.0,
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

        let mut event_context =
            EventContext::new(
                &self.theme,
            );

        self.event_dispatcher.dispatch(
            &root,
            window
                .viewport()
                .logical_bounds(),
            &event,
            &mut event_context,
        );

        if self.primary_button_state
            .take_clicked()
        {
            println!(
                "primary button clicked"
            );
        }

        if event_context
            .redraw_requested()
        {
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

        let mut paint_context =
            PaintContext {
                display_list,

                theme:
                &self.theme,

                typography:
                &self.typography,
            };

        root.paint(
            viewport.logical_bounds(),
            &mut paint_context,
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
                    "ViewKit Button Example",
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