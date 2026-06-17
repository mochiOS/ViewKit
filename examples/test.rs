use viewkit::platform::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use viewkit::platform::linux::LinuxBackend;

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

            PlatformEvent::RedrawRequested => {
                println!("redraw requested");
            }

            PlatformEvent::CloseRequested => {
                println!("close requested");
            }

            _ => {}
        }
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