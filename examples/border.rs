use viewkit::components::{BorderStyle, Card, Padding, Text, VStack};
use viewkit::draw_command::{DisplayList, DrawCommand};
use viewkit::geometry::Size;
use viewkit::layout::{StackAlignment, StackDistribution, StackGap, ViewExt};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use viewkit::renderer::Viewport;
use viewkit::theme::{ShadowStyle, Theme};
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
        let default_border = Card::new().shadow(ShadowStyle::None).content(
            Padding::symmetric(20.0, 16.0).content(
                Text::new("Default — Standard 1px")
                    .font_size(14.0)
                    .line_height(20.0)
                    .weight(600)
                    .color(self.theme.colors.text_primary),
            ),
        );

        let strong_border = Card::new()
            .shadow(ShadowStyle::None)
            .border(BorderStyle::strong(1.0))
            .content(
                Padding::symmetric(20.0, 16.0).content(
                    Text::new("Strong — 1px")
                        .font_size(14.0)
                        .line_height(20.0)
                        .weight(600)
                        .color(self.theme.colors.text_primary),
                ),
            );

        let accent_border = Card::new()
            .shadow(ShadowStyle::None)
            .border(BorderStyle::custom(self.theme.colors.accent, 1.0))
            .content(
                Padding::symmetric(20.0, 16.0).content(
                    Text::new("Accent — 1px")
                        .font_size(14.0)
                        .line_height(20.0)
                        .weight(600)
                        .color(self.theme.colors.text_primary),
                ),
            );

        let thick_accent_border = Card::new()
            .shadow(ShadowStyle::None)
            .border(BorderStyle::custom(self.theme.colors.accent, 2.0))
            .content(
                Padding::symmetric(20.0, 16.0).content(
                    Text::new("Accent — 2px")
                        .font_size(14.0)
                        .line_height(20.0)
                        .weight(600)
                        .color(self.theme.colors.text_primary),
                ),
            );

        let no_border = Card::new()
            .shadow(ShadowStyle::None)
            .border(BorderStyle::None)
            .content(
                Padding::symmetric(20.0, 16.0).content(
                    Text::new("No border")
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
            .child(default_border.width(360.0))
            .child(strong_border.width(360.0))
            .child(accent_border.width(360.0))
            .child(thick_accent_border.width(360.0))
            .child(no_border.width(360.0))
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

            _ => {}
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
