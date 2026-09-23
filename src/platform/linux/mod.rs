mod backend;
#[path = "../windows/gpu_renderer.rs"]
mod gpu_renderer;
mod software_renderer;

pub use backend::{LinuxBackend, LinuxBackendError};
pub use gpu_renderer::{GpuRenderer, GpuRendererError};

#[cfg(target_os = "windows")]
pub type WindowsBackend<A> = LinuxBackend<A>;
#[cfg(target_os = "windows")]
pub type WindowsBackendError = LinuxBackendError;

pub use software_renderer::{SoftwareRenderer, SoftwareRendererError};
