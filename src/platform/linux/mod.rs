mod backend;
mod gpu_renderer;
mod gpu_scene;

pub use backend::{LinuxBackend, LinuxBackendError};
pub use gpu_renderer::{GpuRenderer, GpuRendererError};

#[cfg(target_os = "windows")]
pub type WindowsBackend<A> = LinuxBackend<A>;
#[cfg(target_os = "windows")]
pub type WindowsBackendError = LinuxBackendError;
