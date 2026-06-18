//! プラットフォームから通知されるイベントを定義

use crate::renderer::Viewport;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformEvent {
    Resumed {
        viewport: Viewport,
    },
    Resized {
        viewport: Viewport,
    },
    ScaleFactorChanged {
        viewport: Viewport,
    },
    Scroll {
        delta_x: f32,
        delta_y: f32,
    },
    Focused(bool),
    RedrawRequested,
    CloseRequested,
}