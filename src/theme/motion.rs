use std::time::Duration;

use crate::animation::Easing;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub duration: Duration,
    pub easing: Easing,
}

impl Motion {
    pub const fn new(duration: Duration, easing: Easing) -> Self {
        Self { duration, easing }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionTokens {
    pub omochi: crate::omochi::OmochiMotion,
    pub fast: Motion,
    pub standard: Motion,
    pub slow: Motion,

    pub selection: Motion,
    pub toggle: Motion,

    /// メニュー全体の表示遷移。色やレイアウトの遷移とは分離します。
    pub menu: Motion,
    pub menu_dismiss: Motion,
    pub menu_closed_scale: f32,
}

impl MotionTokens {
    /// すべての状態遷移を即時反映する、動きを抑えた設定です。
    pub const fn reduced(self) -> Self {
        let instant = Motion::new(Duration::ZERO, Easing::Linear);
        Self {
            omochi: crate::omochi::OmochiMotion {
                enabled: false,
                ..self.omochi
            },
            fast: instant,
            standard: instant,
            slow: instant,
            selection: instant,
            toggle: instant,
            menu: instant,
            menu_dismiss: instant,
            menu_closed_scale: 1.0,
        }
    }

    pub const DEFAULT: Self = Self {
        omochi: crate::omochi::OmochiMotion::DEFAULT,
        fast: Motion::new(Duration::from_millis(100), Easing::EaseOutCubic),

        standard: Motion::new(Duration::from_millis(180), Easing::EaseOutCubic),

        slow: Motion::new(Duration::from_millis(280), Easing::EaseInOutCubic),

        selection: Motion::new(Duration::from_millis(180), Easing::EaseOutCubic),

        toggle: Motion::new(Duration::from_millis(180), Easing::EaseOutCubic),
        menu: Motion::new(Duration::from_millis(220), Easing::EaseOutCubic),
        menu_dismiss: Motion::new(Duration::from_millis(140), Easing::EaseInCubic),
        menu_closed_scale: 0.92,
    };
}
