mod event;
mod window;

#[cfg(target_os = "linux")]
pub mod linux;

pub use event::PlatformEvent;
pub use window::{
    PlatformApplication,
    PlatformWindow,
    WindowConfig,
};