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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Escape,
    Tab,
    Enter,
    Space,
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    Character(char),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const SHIFT: u8 = 1 << 0;
    pub const CONTROL: u8 = 1 << 1;
    pub const ALT: u8 = 1 << 2;
    pub const SUPER: u8 = 1 << 3;

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & (Self::SHIFT | Self::CONTROL | Self::ALT | Self::SUPER))
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn shift(self) -> bool {
        self.0 & Self::SHIFT != 0
    }

    pub const fn control(self) -> bool {
        self.0 & Self::CONTROL != 0
    }

    pub const fn alt(self) -> bool {
        self.0 & Self::ALT != 0
    }

    pub const fn super_key(self) -> bool {
        self.0 & Self::SUPER != 0
    }

    pub const fn shortcut(self) -> bool {
        self.control() || self.super_key()
    }
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
    KeyPressed {
        key: Key,
        modifiers: KeyModifiers,
    },
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

#[cfg(test)]
mod tests {
    use super::KeyModifiers;

    #[test]
    fn key_modifiers_mask_unknown_bits_and_identify_shortcuts() {
        let modifiers = KeyModifiers::from_bits(0xff);

        assert_eq!(modifiers.bits(), 0x0f);
        assert!(modifiers.shift());
        assert!(modifiers.control());
        assert!(modifiers.alt());
        assert!(modifiers.super_key());
        assert!(modifiers.shortcut());
    }
}
