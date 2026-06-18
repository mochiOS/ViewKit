use viewkit::components::{
    Background,
    Padding,
    Rectangle,
    RectangleColor,
    Text,
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
        let text = Text::new(
            concat!(
            "おもちは白く、丸く、美味く、幸福を運んでくれます。",
            "猫ははちゃめちゃに可愛く、すべてが愛らしい。",
            "どちらも日々の疲れを癒やす、尊い宝です。尊..."
            ),
        )
            .font_size(18.0)
            .line_height(30.0)
            .weight(600)
            .alignment(TextAlignment::Start)
            .color(Color::BLACK);

        let card = Background::new()
            .background(
                Rectangle::new()
                    .color(
                        RectangleColor::ElevatedSurface,
                    ),
            )
            .content(
                Padding::symmetric(
                    24.0,
                    18.0,
                )
                    .content(text),
            );

        VStack::new()
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                /*
                 * 幅だけを420pxへ固定します。
                 * 高さはTextの折り返し後の測定結果、
                 * Padding、Backgroundの順に伝播します。
                 */
                card.width(420.0),
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