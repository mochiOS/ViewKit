use viewkit::components::{
    Rectangle,
    RectangleColor,
    Text,
    VStack,
    ZStack,
    ZStackAlignment,
};
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::geometry::Size;
use viewkit::layout::{
    StackAlignment,
    StackDistribution,
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

    fn build_root(&self) -> VStack {
        let labeled_rectangle = ZStack::new()
            .alignment(
                ZStackAlignment::Center,
            )
            .child(
                Rectangle::new()
            )
            .child(
                Text::new(
                    "おもちは白く、丸く、美味く、幸福を運んでくれます。\
                    猫ははちゃめちゃに可愛く、すべてが愛らしい存在です。\
                    どちらも日々の疲れを溶かし、世界を少しだけ優しくしてくれる尊い宝です。尊...",
                )
                    .font_size(
                        12.0,
                    )
                    .line_height(
                        36.0,
                    )
                    .weight(
                        600,
                    )
                    .frame(
                        100.0,
                        36.0,
                    ),
            );

        VStack::new()
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                labeled_rectangle.frame(
                    360.0,
                    120.0,
                ),
            )
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
                typography: &self.typography,
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
                    "ViewKit Text Example",
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