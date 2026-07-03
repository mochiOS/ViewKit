//! プラットフォームから通知されるイベントを定義

use crate::renderer::Viewport;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Clone, Debug, PartialEq)]
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
    PointerMoved {
        x: f32,
        y: f32,
    },
    PointerButton {
        button: PointerButton,
        state: ButtonState,
    },
    PointerLeft,
    TextInput {
        text: String,
    },

    Backspace,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    Delete,

    SelectLeft,
    SelectRight,
    SelectHome,
    SelectEnd,
    SelectAll,

    Focused(bool),
    RedrawRequested,
    CloseRequested,
}
