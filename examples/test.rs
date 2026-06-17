use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::geometry::Rect;
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use viewkit::renderer::Viewport;
use viewkit::theme::Color;

struct ExampleApplication;

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

            PlatformEvent::RedrawRequested => {
                println!("redraw requested");
            }

            PlatformEvent::CloseRequested => {
                println!("close requested");
            }
        }
    }

    fn draw(
        &mut self,
        _viewport: Viewport,
        display_list: &mut DisplayList,
    ) {
        display_list.push(DrawCommand::Clear {
            color: Color::from_rgb_hex(0xf5f5f7),
        });

        display_list.push(DrawCommand::FillRect {
            rect: Rect::new(
                40.0,
                40.0,
                180.0,
                100.0,
            ),
            color: Color::from_rgb_hex(0x007aff),
        });

        display_list.push(DrawCommand::FillRoundedRect {
            rect: Rect::new(
                40.0,
                170.0,
                240.0,
                100.0,
            ),
            radius: 18.0,
            color: Color::from_rgb_hex(0x34c759),
        });

        display_list.push(DrawCommand::StrokeRoundedRect {
            rect: Rect::new(
                320.0,
                40.0,
                240.0,
                100.0,
            ),
            radius: 18.0,
            color: Color::from_rgb_hex(0xff3b30),
            width: 4.0,
        });

        display_list.push(DrawCommand::StrokeRect {
            rect: Rect::new(
                320.0,
                170.0,
                240.0,
                100.0,
            ),
            color: Color::from_rgb_hex(0x5856d6),
            width: 4.0,
        });
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = LinuxBackend::new(
        ExampleApplication,
        WindowConfig {
            title: String::from("ViewKit Drawing Example"),
            size: viewkit::geometry::Size::new(
                640.0,
                360.0,
            ),
            resizable: true,
        },
    );

    backend.run()?;

    Ok(())
}