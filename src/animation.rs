use std::time::{Duration, Instant};

use crate::theme::Color;

pub const DEFAULT_FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
    Linear,

    #[default]
    EaseOutCubic,

    EaseInOutCubic,
}

impl Easing {
    pub fn apply(self, progress: f32) -> f32 {
        let progress = sanitize_progress(progress);

        match self {
            Self::Linear => progress,

            Self::EaseOutCubic => {
                let inverse = 1.0 - progress;

                1.0 - inverse * inverse * inverse
            }

            Self::EaseInOutCubic => {
                if progress < 0.5 {
                    4.0 * progress * progress * progress
                } else {
                    let value = -2.0 * progress + 2.0;

                    1.0 - value * value * value / 2.0
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transition<T> {
    pub from: T,
    pub to: T,
    pub started_at: Instant,
}

impl<T> Transition<T> {
    pub const fn new(from: T, to: T, started_at: Instant) -> Self {
        Self {
            from,
            to,
            started_at,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Animation {
    started_at: Instant,
    duration: Duration,
    frame_interval: Duration,
    easing: Easing,
}

impl Animation {
    pub const fn new(started_at: Instant, duration: Duration) -> Self {
        Self {
            started_at,
            duration,
            frame_interval: DEFAULT_FRAME_INTERVAL,
            easing: Easing::EaseOutCubic,
        }
    }

    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn frame_interval(mut self, frame_interval: Duration) -> Self {
        self.frame_interval = if frame_interval.is_zero() {
            DEFAULT_FRAME_INTERVAL
        } else {
            frame_interval
        };

        self
    }

    pub fn sample(self, now: Instant) -> AnimationSample {
        if self.duration.is_zero() {
            return AnimationSample {
                progress: 1.0,
                linear_progress: 1.0,
                finished: true,
                next_redraw_at: None,
            };
        }

        let elapsed = now.saturating_duration_since(self.started_at);

        if elapsed >= self.duration {
            return AnimationSample {
                progress: self.easing.apply(1.0),
                linear_progress: 1.0,
                finished: true,
                next_redraw_at: None,
            };
        }

        let linear_progress = (elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);

        let animation_end = self.started_at + self.duration;

        let requested_frame = now + self.frame_interval;

        let next_redraw_at = if requested_frame < animation_end {
            requested_frame
        } else {
            animation_end
        };

        AnimationSample {
            progress: self.easing.apply(linear_progress),
            linear_progress,
            finished: false,
            next_redraw_at: Some(next_redraw_at),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AnimationSample {
    pub progress: f32,
    pub linear_progress: f32,
    pub finished: bool,
    pub next_redraw_at: Option<Instant>,
}

pub trait Interpolate: Sized {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self;
}

pub fn interpolate<T>(from: T, to: T, progress: f32) -> T
where
    T: Interpolate,
{
    T::interpolate(from, to, sanitize_progress(progress))
}

impl Interpolate for f32 {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self {
        from + (to - from) * progress
    }
}

impl Interpolate for f64 {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self {
        from + (to - from) * progress as f64
    }
}

impl Interpolate for Color {
    fn interpolate(from: Self, to: Self, progress: f32) -> Self {
        Self::rgba(
            interpolate_channel(from.red, to.red, progress),
            interpolate_channel(from.green, to.green, progress),
            interpolate_channel(from.blue, to.blue, progress),
            interpolate_channel(from.alpha, to.alpha, progress),
        )
    }
}

fn interpolate_channel(from: u8, to: u8, progress: f32) -> u8 {
    interpolate(from as f32, to as f32, progress)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn sanitize_progress(progress: f32) -> f32 {
    if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
