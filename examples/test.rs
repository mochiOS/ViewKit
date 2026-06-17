use viewkit::components::Rectangle;
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::geometry::{
    Rect,
    Size,
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
}

impl PlatformApplication for ExampleApplication {
    fn handle_event(
        &mut self,
        event: PlatformEvent,
        _window: &dyn PlatformWindow,
    ) {
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

            PlatformEvent::RedrawRequested => {}

            PlatformEvent::CloseRequested => {
                println!("close requested");
            }
        }
    }

    fn draw(
        &mut self,
        viewport: Viewport,
        display_list: &mut DisplayList,
    ) {
        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        let mut context = PaintContext {
            display_list,
            theme: &self.theme,
            typography: &self.typography,
        };

        let blue_rectangle = Rectangle::new(
            self.theme.colors.accent,
        );

        blue_rectangle.paint(
            Rect::new(
                40.0,
                40.0,
                180.0,
                100.0,
            ),
            &mut context,
        );

        let green_rectangle = Rectangle::rounded(
            Color::from_rgb_hex(0x34c759),
            self.theme.radius.large,
        );

        green_rectangle.paint(
            Rect::new(
                40.0,
                170.0,
                240.0,
                100.0,
            ),
            &mut context,
        );

        let centered_width = 260.0;
        let centered_height = 120.0;

        let centered_x =
            (viewport.logical_size.width - centered_width)
                / 2.0;

        let centered_y =
            viewport.logical_size.height
                - centered_height
                - 40.0;

        let centered_rectangle = Rectangle::rounded(
            self.theme.colors.surface,
            self.theme.radius.extra_large,
        );

        centered_rectangle.paint(
            Rect::new(
                centered_x,
                centered_y,
                centered_width,
                centered_height,
            ),
            &mut context,
        );

        context.display_list.push(
            DrawCommand::StrokeRoundedRect {
                rect: Rect::new(
                    centered_x,
                    centered_y,
                    centered_width,
                    centered_height,
                ),
                radius: self.theme.radius.extra_large,
                color: self.theme.colors.border,
                width: 2.0,
            },
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let application = ExampleApplication::new();

    let backend = LinuxBackend::new(
        application,
        WindowConfig {
            title: String::from(
                "ViewKit Component Example",
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