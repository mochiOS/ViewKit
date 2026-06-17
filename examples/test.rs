use viewkit::components::{
    HStack,
    Rectangle,
    RectangleColor,
    VStack,
};
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::geometry::{
    Rect,
    Size,
};
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
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
        }
    }
}

impl PlatformApplication for ExampleApplication {
    fn handle_event(
        &mut self,
        event: PlatformEvent,
        _window: &dyn PlatformWindow,
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

        let rectangle = Rectangle::new();

        let horizontal_stack = HStack::new()
            .gap(StackGap::Medium)
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                rectangle
                    .color(
                        RectangleColor::Accent,
                    )
                    .frame(
                        130.0,
                        90.0,
                    ),
            )
            .child(
                rectangle
                    .color(
                        RectangleColor::Destructive,
                    )
                    .frame(
                        130.0,
                        90.0,
                    ),
            );

        let vertical_stack = VStack::new()
            .gap(StackGap::Large)
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                rectangle
                    .color(
                        RectangleColor::ElevatedSurface,
                    )
                    .frame(
                        300.0,
                        120.0,
                    ),
            )
            .child(
                horizontal_stack.frame(
                    300.0,
                    90.0,
                ),
            );

        let mut context = PaintContext {
            display_list,
            theme: &self.theme,
            typography: &self.typography,
        };

        vertical_stack.paint(
            Rect::new(
                0.0,
                0.0,
                viewport.logical_size.width,
                viewport.logical_size.height,
            ),
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

    let backend = LinuxBackend::new(
        application,
        WindowConfig {
            title: String::from(
                "ViewKit Stack Example",
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