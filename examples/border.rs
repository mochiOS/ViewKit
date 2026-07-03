use viewkit::components::{Background, Card, Padding, Text, VStack};
use viewkit::draw_command::{DisplayList, DrawCommand};
use viewkit::geometry::Size;
use viewkit::layout::{StackAlignment, StackDistribution, StackGap, ViewExt};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use viewkit::renderer::Viewport;
use viewkit::theme::Theme;
use viewkit::typography::{TextMeasurer, Typography};
use viewkit::view::{PaintContext, View};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),
        }
    }

    fn build_root(&self) -> VStack {
        let standard_border = Background::new().background(Card::new()).content(
            Padding::symmetric(20.0, 16.0).content(
                Text::new("Standard border")
                    .font_size(14.0)
                    .line_height(20.0)
                    .weight(600)
                    .color(self.theme.colors.text_primary),
            ),
        );

        let strong_border = Background::new().background(Card::new()).content(
            Padding::symmetric(20.0, 16.0).content(
                Text::new("Strong border")
                    .font_size(14.0)
                    .line_height(20.0)
                    .weight(600)
                    .color(self.theme.colors.text_primary),
            ),
        );

        let custom_border = Background::new().background(Card::new()).content(
            Padding::symmetric(20.0, 16.0).content(
                Text::new("Custom accent border")
                    .font_size(14.0)
                    .line_height(20.0)
                    .weight(600)
                    .color(self.theme.colors.text_primary),
            ),
        );

        VStack::new()
            .gap(StackGap::Large)
            .alignment(StackAlignment::Center)
            .distribution(StackDistribution::Center)
            .child(standard_border.width(360.0))
            .child(strong_border.width(360.0))
            .child(custom_border.width(360.0))
    }
}

impl PlatformApplication for ExampleApplication {
    fn handle_event(&mut self, event: PlatformEvent, _window: &dyn PlatformWindow) {
        match event {
            PlatformEvent::Resumed { viewport } => {
                println!("resumed: {viewport:?}");
            }

            PlatformEvent::Resized { viewport } => {
                println!("resized: {viewport:?}");
            }

            PlatformEvent::ScaleFactorChanged { viewport } => {
                println!("scale factor changed: {viewport:?}");
            }

            PlatformEvent::Focused(focused) => {
                println!("focused: {focused}");
            }

            PlatformEvent::CloseRequested => {
                println!("close requested");
            }

            PlatformEvent::PointerMoved { .. }
            | PlatformEvent::PointerButton { .. }
            | PlatformEvent::PointerLeft
            | PlatformEvent::Scroll { .. }
            | PlatformEvent::RedrawRequested => {}
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        let root = self.build_root();

        let mut context = PaintContext {
            display_list,
            theme: &self.theme,
            typography: &self.typography,
            text_measurer: &mut self.text_measurer,
        };

        root.paint(viewport.logical_bounds(), &mut context);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let application = ExampleApplication::new();

    let backend = LinuxBackend::new(
        application,
        WindowConfig {
            title: String::from("ViewKit Border Example"),
            size: Size::new(720.0, 520.0),
            resizable: true,
        },
    );

    backend.run()?;

    Ok(())
}
