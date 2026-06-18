use viewkit::components::{
    Button,
    ButtonColor,
    ButtonInteractionState,
    Text,
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
use viewkit::theme::{
    Color,
    Theme,
};
use viewkit::typography::{
    TextAlignment,
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

    event_dispatcher: EventDispatcher,

    primary_button_state:
        ButtonInteractionState,

    destructive_button_state:
        ButtonInteractionState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher:
            EventDispatcher::new(),

            primary_button_state:
            ButtonInteractionState::new(),

            destructive_button_state:
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
                        "続行",
                    )
                        .font_size(
                            17.0,
                        )
                        .line_height(
                            26.0,
                        )
                        .weight(
                            600,
                        )
                        .alignment(
                            TextAlignment::Center,
                        )
                        .color(
                            Color::WHITE,
                        )
                        .frame(
                            180.0,
                            26.0,
                        ),
                );

        let destructive_button =
            Button::new(
                self.destructive_button_state
                    .clone(),
            )
                .color(
                    ButtonColor::Destructive,
                )
                .content(
                    Text::new(
                        "削除",
                    )
                        .font_size(
                            17.0,
                        )
                        .line_height(
                            26.0,
                        )
                        .weight(
                            600,
                        )
                        .alignment(
                            TextAlignment::Center,
                        )
                        .color(
                            Color::WHITE,
                        )
                        .frame(
                            180.0,
                            26.0,
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
                    56.0,
                ),
            )
            .child(
                destructive_button.frame(
                    240.0,
                    56.0,
                ),
            )
    }
}

impl PlatformApplication for ExampleApplication {
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

        if self.primary_button_state
            .take_clicked()
        {
            println!(
                "続行ボタンがクリックされました"
            );
        }

        if self.destructive_button_state
            .take_clicked()
        {
            println!(
                "削除ボタンがクリックされました"
            );
        }

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
                    "scale factor changed: {viewport:?}"
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
                theme: &self.theme,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
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