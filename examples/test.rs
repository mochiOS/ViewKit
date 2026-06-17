use viewkit::platform::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use viewkit::platform::linux::LinuxBackend;
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
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
        println!("{event:?}");
    }

    fn draw(
        &mut self,
        _viewport: Viewport,
        display_list: &mut DisplayList,
    ) {
        display_list.push(DrawCommand::Clear {
            color: Color::from_rgb_hex(0x4a78c7),
        });
    }
}

fn main() -> Result<
    (),
    Box<dyn std::error::Error>,
> {
    let backend = LinuxBackend::new(
        ExampleApplication,
        WindowConfig {
            title: String::from("ViewKit Example"),
            ..WindowConfig::default()
        },
    );

    backend.run()?;

    Ok(())
}