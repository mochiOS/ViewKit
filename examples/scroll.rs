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
use viewkit::typography::Typography;
use viewkit::view::{
    PaintContext,
    View,
};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
    scroll_state: ScrollState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,

            typography:
            Typography::DEFAULT,

            scroll_state:
            ScrollState::new(),
        }
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

            PlatformEvent::Scroll {
                delta_x,
                delta_y,
            } => {
                self.scroll_state.scroll_by(
                    delta_x,
                    delta_y,
                );

                window.request_redraw();
            }

            PlatformEvent::RedrawRequested => {}

            PlatformEvent::CloseRequested => {
                println!(
                    "close requested"
                );
            }
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

        let scroll = Scroll::new(
            self.scroll_state.clone(),
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

        let card = Background::new()
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

        let root = VStack::new()
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
            );

        let mut context =
            PaintContext {
                display_list,

                theme:
                &self.theme,

                typography:
                &self.typography,
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