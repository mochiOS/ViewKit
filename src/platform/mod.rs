mod event;
mod window;

#[cfg(target_os = "linux")]
pub mod linux;

pub use event::{ButtonState, PlatformEvent, PointerButton};
pub use window::{PlatformApplication, PlatformWindow, WindowConfig};
