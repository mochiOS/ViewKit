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
    pub fast: Motion,
    pub standard: Motion,
    pub slow: Motion,

    pub selection: Motion,
    pub toggle: Motion,
}

impl MotionTokens {
    pub const DEFAULT: Self = Self {
        fast: Motion::new(Duration::from_millis(100), Easing::EaseOutCubic),

        standard: Motion::new(Duration::from_millis(180), Easing::EaseOutCubic),

        slow: Motion::new(Duration::from_millis(280), Easing::EaseInOutCubic),

        selection: Motion::new(Duration::from_millis(180), Easing::EaseOutCubic),

        toggle: Motion::new(Duration::from_millis(180), Easing::EaseOutCubic),
    };
}
