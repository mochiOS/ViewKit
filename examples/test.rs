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

        let width = 280.0;
        let height = 160.0;

        let x = (viewport.logical_size.width - width) / 2.0;
        let y = (viewport.logical_size.height - height) / 2.0;

        let rectangle = Rectangle::new();

        rectangle.paint(
            Rect::new(
                x,
                y,
                width,
                height,
            ),
            &mut context,
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