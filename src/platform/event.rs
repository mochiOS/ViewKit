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

    Focused(bool),

    RedrawRequested,

    CloseRequested,
}