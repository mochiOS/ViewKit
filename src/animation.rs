use std::time::{Duration, Instant};

use crate::theme::Color;

pub const DEFAULT_FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
    Linear,

    #[default]
    EaseOutCubic,

    EaseInOutCubic,
    EaseInCubic,
}

impl Easing {
    pub fn apply(self, progress: f32) -> f32 {
        let progress = sanitize_progress(progress);

        match self {
            Self::Linear => progress,
            Self::EaseInCubic => progress * progress * progress,

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

/// 開閉を途中で反転しても、表示中の位置から連続して遷移する状態です。
/// パネルの所有側で保持し、毎フレームのprogressを位置や透明度へ適用します。
#[derive(Clone, Copy, Debug, Default)]
pub struct VisibilityTransition {
    from: f32,
    visible: bool,
    started_at: Option<Instant>,
    motion: Option<crate::theme::Motion>,
}

impl VisibilityTransition {
    /// 0（非表示）から1（表示）までの進行度と、必要な次回描画時刻を返します。
    pub fn sample(
        &mut self,
        visible: bool,
        motion: crate::theme::Motion,
        now: Instant,
    ) -> (f32, Option<Instant>) {
        if motion.duration.is_zero() {
            self.visible = visible;
            self.from = if visible { 1.0 } else { 0.0 };
            self.started_at = None;
            self.motion = None;
            return (self.from, None);
        }
        if self.visible != visible {
            self.from = self.current(self.motion.unwrap_or(motion), now).0;
            self.visible = visible;
            self.started_at = Some(now);
            self.motion = Some(motion);
        }
        let result = self.current(self.motion.unwrap_or(motion), now);
        if result.1.is_none() {
            self.from = result.0;
            self.started_at = None;
            self.motion = None;
        }
        result
    }

    fn current(&self, motion: crate::theme::Motion, now: Instant) -> (f32, Option<Instant>) {
        let target = if self.visible { 1.0 } else { 0.0 };
        let Some(started) = self.started_at else {
            return (target, None);
        };
        let sample = Animation::new(started, motion.duration)
            .easing(motion.easing)
            .sample(now);
        (
            interpolate(self.from, target, sample.progress),
            sample.next_redraw_at,
        )
    }
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

/// Shared visual transition for omochi controls. Retargeting starts at the
/// currently presented appearance, never at a stale interaction endpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AppearanceTransition {
    segment: Option<AppearanceSegment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AppearanceSegment {
    from: crate::theme::ControlAppearance,
    to: crate::theme::ControlAppearance,
    started_at: Instant,
}

impl AppearanceTransition {
    pub(crate) fn sample(
        &mut self,
        target: crate::theme::ControlAppearance,
        motion: crate::theme::Motion,
        now: Instant,
        immediate: bool,
    ) -> (crate::theme::ControlAppearance, Option<Instant>) {
        let Some(mut segment) = self.segment else {
            self.segment = Some(AppearanceSegment {
                from: target,
                to: target,
                started_at: now,
            });
            return (target, None);
        };
        if immediate || motion.duration.is_zero() {
            self.segment = Some(AppearanceSegment {
                from: target,
                to: target,
                started_at: now,
            });
            return (target, None);
        }
        if segment.to != target {
            let (presented, _) = Self::sample_segment(segment, motion, now);
            segment = AppearanceSegment {
                from: presented,
                to: target,
                started_at: now,
            };
            self.segment = Some(segment);
        }
        Self::sample_segment(segment, motion, now)
    }

    fn sample_segment(
        segment: AppearanceSegment,
        motion: crate::theme::Motion,
        now: Instant,
    ) -> (crate::theme::ControlAppearance, Option<Instant>) {
        if segment.from == segment.to {
            return (segment.to, None);
        }
        let sample = Animation::new(segment.started_at, motion.duration)
            .easing(motion.easing)
            .sample(now);
        (
            crate::theme::ControlAppearance {
                background: interpolate(
                    segment.from.background,
                    segment.to.background,
                    sample.progress,
                ),
                border: interpolate(segment.from.border, segment.to.border, sample.progress),
                foreground: interpolate(
                    segment.from.foreground,
                    segment.to.foreground,
                    sample.progress,
                ),
            },
            sample.next_redraw_at,
        )
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    use crate::theme::{ControlAppearance, Motion};

    fn appearance(color: Color) -> ControlAppearance {
        ControlAppearance {
            background: color,
            border: color,
            foreground: color,
        }
    }

    #[test]
    fn interrupted_hover_reverses_from_presented_color_and_stops_when_settled() {
        let start = Instant::now();
        let motion = Motion::new(Duration::from_millis(100), Easing::EaseOutCubic);
        let rest = appearance(Color::BLACK);
        let hover = appearance(Color::WHITE);
        let mut transition = AppearanceTransition::default();
        assert_eq!(transition.sample(rest, motion, start, false), (rest, None));
        transition.sample(hover, motion, start, false);
        let halfway = start + Duration::from_millis(50);
        let (presented, next) = transition.sample(hover, motion, halfway, false);
        assert!(presented.background.red > 0 && presented.background.red < 255);
        assert!(next.is_some());
        assert_eq!(transition.sample(rest, motion, halfway, false).0, presented);
        assert_eq!(
            transition.sample(rest, motion, halfway + motion.duration, false),
            (rest, None)
        );
    }

    #[test]
    fn press_feedback_and_reduced_motion_apply_immediately() {
        let now = Instant::now();
        let normal = crate::theme::MotionTokens::DEFAULT;
        let rest = appearance(Color::WHITE);
        let pressed = appearance(Color::BLACK);
        let mut transition = AppearanceTransition::default();
        transition.sample(rest, normal.fast, now, false);
        assert_eq!(
            transition.sample(pressed, normal.fast, now, true),
            (pressed, None)
        );
        assert_eq!(
            transition.sample(rest, normal.reduced().fast, now, false),
            (rest, None)
        );
    }

    #[test]
    fn visibility_can_reverse_midflight_and_reduced_motion_finishes_immediately() {
        let now = Instant::now();
        let motion = crate::theme::MotionTokens::DEFAULT;
        let mut visibility = VisibilityTransition::default();
        assert_eq!(visibility.sample(false, motion.standard, now), (0.0, None));
        assert_eq!(visibility.sample(true, motion.standard, now).0, 0.0);
        let halfway = now + motion.standard.duration / 2;
        let amount = visibility.sample(true, motion.standard, halfway).0;
        assert!(amount > 0.0 && amount < 1.0);
        assert_eq!(visibility.sample(false, motion.standard, halfway).0, amount);
        assert_eq!(
            visibility.sample(false, motion.standard, halfway + motion.standard.duration),
            (0.0, None)
        );
        assert_eq!(
            visibility.sample(
                true,
                motion.reduced().standard,
                halfway + motion.standard.duration
            ),
            (1.0, None)
        );
    }
    #[test]
    fn menu_reversal_keeps_position_with_different_enter_and_exit_curves() {
        let now = Instant::now();
        let motion = crate::theme::MotionTokens::DEFAULT;
        let mut transition = VisibilityTransition::default();
        transition.sample(true, motion.menu, now);
        let at = now + motion.menu.duration / 2;
        let opened = transition.sample(true, motion.menu, at).0;
        assert_eq!(transition.sample(false, motion.menu_dismiss, at).0, opened);
        let at = at + motion.menu_dismiss.duration / 2;
        let closing = transition.sample(false, motion.menu_dismiss, at).0;
        assert!(closing > 0.0 && closing < opened);
        assert_eq!(transition.sample(true, motion.menu, at).0, closing);
        assert_eq!(
            transition.sample(true, motion.menu, at + motion.menu.duration),
            (1.0, None)
        );
        transition.sample(false, motion.menu_dismiss, at + motion.menu.duration);
        assert_eq!(
            transition.sample(
                false,
                motion.reduced().menu_dismiss,
                at + motion.menu.duration
            ),
            (0.0, None)
        );
    }
}
