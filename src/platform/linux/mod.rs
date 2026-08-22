mod backend;
mod software_renderer;

pub use backend::{LinuxBackend, LinuxBackendError};

#[cfg(target_os = "windows")]
pub type WindowsBackend<A> = LinuxBackend<A>;
#[cfg(target_os = "windows")]
pub type WindowsBackendError = LinuxBackendError;

pub use software_renderer::{SoftwareRenderer, SoftwareRendererError};
