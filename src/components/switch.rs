use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};
use crate::accessibility::AccessibilityRole;
use crate::animation::{Transition, interpolate};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::omochi::moving::SwitchThumb;
use crate::platform::PointerButton;
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

#[derive(Clone, Copy)]
struct SwitchMetrics {
    track_width: f32,
    track_height: f32,
    knob_width: f32,
    knob_height: f32,
    knob_inset: f32,
    drag_threshold: f32,
    hit_padding: f32,
}

impl SwitchMetrics {
    fn from_theme(theme: &Theme) -> Self {
        let layout = theme.layout;
        let maximum_width = (layout.switch_track_width - layout.switch_knob_inset * 2.0).max(1.0);
        let maximum_height = (layout.switch_track_height - layout.switch_knob_inset * 2.0).max(1.0);
        let thumb = layout.control_thumb_size(false);
        let knob_height = thumb.height.min(maximum_height);
        let knob_width = thumb.width.min(maximum_width);
        Self {
            track_width: layout.switch_track_width,
            track_height: layout.switch_track_height,
            knob_width,
            knob_height,
            knob_inset: layout.switch_knob_inset,
            drag_threshold: layout.switch_drag_threshold,
            hit_padding: layout.switch_hit_padding,
        }
    }
}

#[derive(Default)]
struct SwitchDragInner {
    mark_bounds: Option<Rect>,

    tracking: bool,
    drag_candidate: bool,
    dragging: bool,

    press_x: f32,
    drag_offset_x: f32,

    drag_position: Option<f32>,
}

#[derive(Clone, Default)]
struct SwitchDragState {
    inner: Rc<RefCell<SwitchDragInner>>,
}

/// Shared switch interaction and omochi state, retained across view reconstruction.
#[derive(Clone, Default)]
pub struct SwitchInteractionState {
    interaction: ButtonInteractionState,
    drag: SwitchDragState,
    thumb: Rc<RefCell<SwitchThumb>>,
}
impl SwitchInteractionState {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Switch {
    checked: Binding<bool>,
    label: Option<String>,
    enabled: bool,
    interaction: ButtonInteractionState,
    thumb: Rc<RefCell<SwitchThumb>>,
    drag: SwitchDragState,
}

impl Switch {
    pub fn new(checked: Binding<bool>) -> Self {
        Self::with_interaction(checked, SwitchInteractionState::new())
    }

    pub fn with_interaction(checked: Binding<bool>, state: SwitchInteractionState) -> Self {
        Self {
            checked,
            label: None,
            enabled: true,
            interaction: state.interaction,
            drag: state.drag,
            thumb: state.thumb,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked.get()
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    fn button(&self, theme: &Theme) -> Button {
        let metrics = SwitchMetrics::from_theme(theme);
        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small);

        if let Some(label) = self.label.as_ref() {
            content = content.child(
                Text::label(label.clone())
                    .accessibility_hidden(true)
                    .color(if self.enabled {
                        theme.switch.foreground
                    } else {
                        theme.switch.disabled_foreground
                    })
                    .layout()
                    .flex_shrink(0.0),
            );
        }

        content = content.child(
            SwitchMark {
                checked: self.checked.get(),
                transition: self.checked.transition(),
                enabled: self.enabled,
                interaction: self.interaction.clone(),
                thumb: self.thumb.clone(),
                drag: self.drag.clone(),
            }
            .frame(metrics.track_width, metrics.track_height)
            .flex_shrink(0.0),
        );

        Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: theme.switch.interaction_background,
                hovered_background: theme.switch.interaction_background,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.switch.foreground,
            })
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::all(theme.switch.content_padding).content(content))
            .accessibility_role(AccessibilityRole::Switch)
            .accessibility_checked(self.checked.get())
            .accessibility_label_option(self.label.clone())
    }

    fn handle_switch_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        let metrics = SwitchMetrics::from_theme(context.theme());

        match event {
            ViewEvent::KeyPressed {
                key: crate::platform::Key::Enter | crate::platform::Key::Space,
                ..
            } if self.interaction.is_focused() => {
                self.checked.set(!self.checked.get());
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !bounds.contains(*position) {
                    return EventResult::Ignored;
                }

                let checked_position = bool_position(self.checked.get());

                let mut drag = self.drag.inner.borrow_mut();

                let mark_bounds = drag.mark_bounds;

                let drag_candidate = mark_bounds
                    .map(|mark_bounds| {
                        mark_bounds
                            .expanded(metrics.hit_padding)
                            .contains(*position)
                    })
                    .unwrap_or(false);

                let drag_offset_x = mark_bounds
                    .filter(|mark_bounds| {
                        knob_bounds_at(*mark_bounds, metrics.knob_width, checked_position, metrics)
                            .expanded(metrics.hit_padding)
                            .contains(*position)
                    })
                    .map(|mark_bounds| {
                        position.x
                            - knob_center_x(
                                mark_bounds,
                                metrics.knob_width,
                                checked_position,
                                metrics,
                            )
                    })
                    .unwrap_or(0.0);

                drag.tracking = true;
                drag.drag_candidate = drag_candidate;
                drag.dragging = false;
                drag.press_x = position.x;
                drag.drag_offset_x = drag_offset_x;
                drag.drag_position = Some(checked_position);

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerMoved { position } => {
                let tracking = self.drag.inner.borrow().tracking;

                if !tracking {
                    return EventResult::Ignored;
                }

                {
                    let mut drag = self.drag.inner.borrow_mut();

                    if !drag.dragging {
                        let moved = (position.x - drag.press_x).abs();

                        if drag.drag_candidate && moved >= metrics.drag_threshold {
                            drag.dragging = true;
                        }
                    }

                    if drag.dragging
                        && let Some(mark_bounds) = drag.mark_bounds
                    {
                        drag.drag_position = Some(drag_progress_from_pointer(
                            mark_bounds,
                            position.x,
                            drag.drag_offset_x,
                            metrics,
                        ));
                    }
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let release = {
                    let mut drag = self.drag.inner.borrow_mut();

                    if !drag.tracking {
                        None
                    } else {
                        let was_dragging = drag.dragging;

                        let final_position = if was_dragging {
                            drag.mark_bounds
                                .map(|mark_bounds| {
                                    drag_progress_from_pointer(
                                        mark_bounds,
                                        position.x,
                                        drag.drag_offset_x,
                                        metrics,
                                    )
                                })
                                .or(drag.drag_position)
                                .unwrap_or_else(|| bool_position(self.checked.get()))
                        } else {
                            bool_position(self.checked.get())
                        };

                        drag.tracking = false;
                        drag.drag_candidate = false;
                        drag.dragging = false;
                        drag.press_x = 0.0;
                        drag.drag_offset_x = 0.0;
                        drag.drag_position = None;

                        Some((was_dragging, final_position >= 0.5))
                    }
                };

                let Some((was_dragging, drag_checked)) = release else {
                    return EventResult::Ignored;
                };

                if was_dragging {
                    self.checked.set_if_changed(drag_checked);
                } else if bounds.contains(*position) {
                    self.checked.set(!self.checked.get());
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                if self.drag.inner.borrow().tracking {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::FocusChanged { focused: false } => {
                let _final_position = {
                    let mut drag = self.drag.inner.borrow_mut();

                    if !drag.tracking {
                        return EventResult::Ignored;
                    }

                    let position = drag
                        .drag_position
                        .unwrap_or_else(|| bool_position(self.checked.get()));

                    drag.tracking = false;
                    drag.drag_candidate = false;
                    drag.dragging = false;
                    drag.drag_position = None;
                    drag.drag_offset_x = 0.0;

                    position
                };

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            _ => EventResult::Ignored,
        }
    }
}

impl View for Switch {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.button(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if !self.enabled {
            *self.drag.inner.borrow_mut() = SwitchDragInner::default();
        }
        self.button(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let button_result = self
            .button(context.theme)
            .handle_event(bounds, event, context);

        let switch_result = self.handle_switch_event(bounds, event, context);

        match switch_result {
            EventResult::Consumed => EventResult::Consumed,
            EventResult::Ignored => button_result,
        }
    }
}

struct SwitchMark {
    checked: bool,
    transition: Option<Transition<bool>>,
    enabled: bool,
    interaction: ButtonInteractionState,
    thumb: Rc<RefCell<SwitchThumb>>,
    drag: SwitchDragState,
}

impl View for SwitchMark {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let metrics = SwitchMetrics::from_theme(context.theme);
        constraints.constrain(Size::new(metrics.track_width, metrics.track_height))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.drag.inner.borrow_mut().mark_bounds = Some(bounds);

        let dragging = self.drag.inner.borrow().dragging;
        let hovered = self.interaction.is_hovered();
        let pressed = self.interaction.is_pressed() || dragging;

        let now = Instant::now();
        let metrics = SwitchMetrics::from_theme(context.theme);
        let mut motion = context.theme.motion.omochi;
        motion.enabled &= self.enabled;
        let progress = self
            .drag
            .inner
            .borrow()
            .drag_position
            .unwrap_or_else(|| bool_position(self.checked));
        let left = metrics.knob_inset + metrics.knob_width * 0.5;
        let travel = (bounds.size.width - metrics.knob_inset * 2.0 - metrics.knob_width).max(0.0);
        let target = left + travel * progress;
        let mut thumb = self.thumb.borrow_mut();
        if let Some(transition) = self
            .transition
            .filter(|t| t.to == self.checked && t.from != t.to)
        {
            // A default-constructed switch can resume a bound-value transition as well.
            if now.saturating_duration_since(transition.started_at)
                < context.theme.motion.toggle.duration
            {
                thumb.initialize(
                    left + travel * bool_position(transition.from),
                    transition.started_at,
                );
            }
        }
        let next = thumb.sample(
            target,
            pressed,
            dragging,
            metrics.knob_height / 18.0,
            motion,
            now,
        );
        if let Some(next) = next {
            context.request_redraw_in_at(bounds.expanded(16.0), next);
        }
        let position = if travel > 0.0 {
            ((thumb.center - left) / travel).clamp(0.0, 1.0)
        } else {
            progress
        };
        let track_color = self.track_color(context.theme, hovered, pressed, position);
        Rectangle::new()
            .color(RectangleColor::Custom(track_color))
            .radius(CornerRadius::Full)
            .paint(bounds, context);
        let knob_bounds = Rect::new(
            bounds.origin.x + thumb.center - metrics.knob_width * 0.5,
            bounds.origin.y + (bounds.size.height - metrics.knob_height) * 0.5,
            metrics.knob_width,
            metrics.knob_height,
        );

        let knob_color = if self.enabled {
            context.theme.switch.knob
        } else {
            context.theme.switch.disabled_knob
        };

        let knob_shadow = if self.enabled {
            context.theme.switch.knob_shadow
        } else {
            ShadowStyle::None
        };

        if knob_shadow != ShadowStyle::None {
            Rectangle::new()
                .color(RectangleColor::Custom(Color::TRANSPARENT))
                .radius(CornerRadius::Full)
                .shadow(knob_shadow)
                .paint(knob_bounds, context);
        }
        if thumb.stretch < 0.001 && thumb.press < 0.001 {
            Rectangle::new()
                .color(RectangleColor::Custom(knob_color))
                .radius(CornerRadius::Full)
                .paint(knob_bounds, context);
            return;
        }
        let points = thumb
            .contour(
                bounds.size.height * 0.5,
                metrics.knob_width,
                metrics.knob_height,
            )
            .into_iter()
            .map(|p| crate::geometry::Point::new(p.x + bounds.origin.x, p.y + bounds.origin.y))
            .collect();
        context.display_list.push(DrawCommand::DrawContour {
            points,
            fill: knob_color,
            stroke: Color::TRANSPARENT,
            stroke_width: 0.0,
        });
    }
}

impl SwitchMark {
    fn track_color(&self, theme: &Theme, hovered: bool, pressed: bool, position: f32) -> Color {
        let off_color = if pressed {
            theme.switch.off_pressed
        } else if hovered {
            theme.switch.off_hovered
        } else {
            theme.switch.off
        };

        let on_color = if pressed {
            theme.switch.on_pressed
        } else if hovered {
            theme.switch.on_hovered
        } else {
            theme.switch.on
        };

        let color = interpolate(off_color, on_color, position);

        if self.enabled {
            color
        } else {
            with_opacity(color, theme.switch.disabled_opacity)
        }
    }
}

fn with_opacity(color: Color, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };

    color.with_alpha((color.alpha as f32 * opacity).round() as u8)
}

fn bool_position(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

fn knob_center_x(bounds: Rect, knob_width: f32, progress: f32, metrics: SwitchMetrics) -> f32 {
    let left = bounds.origin.x + metrics.knob_inset + knob_width / 2.0;

    let right = bounds.origin.x + bounds.size.width - metrics.knob_inset - knob_width / 2.0;

    interpolate(left, right, progress.clamp(0.0, 1.0))
}

fn knob_bounds_at(bounds: Rect, knob_width: f32, progress: f32, metrics: SwitchMetrics) -> Rect {
    let center_x = knob_center_x(bounds, knob_width, progress, metrics);

    Rect::new(
        center_x - knob_width / 2.0,
        bounds.origin.y + (bounds.size.height - metrics.knob_height) / 2.0,
        knob_width,
        metrics.knob_height,
    )
}

fn drag_progress_from_pointer(
    bounds: Rect,
    pointer_x: f32,
    drag_offset_x: f32,
    metrics: SwitchMetrics,
) -> f32 {
    let knob_width = metrics.knob_width;

    let left = bounds.origin.x + metrics.knob_inset + knob_width / 2.0;

    let right = bounds.origin.x + bounds.size.width - metrics.knob_inset - knob_width / 2.0;

    let width = right - left;

    if width <= 0.0 {
        return 0.0;
    }

    ((pointer_x - drag_offset_x - left) / width).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::SwitchMetrics;
    use crate::theme::Theme;

    #[test]
    fn dragging_commits_on_release_and_painting_never_changes_the_bound_value() {
        use super::*;
        use crate::{
            draw_command::DisplayList, platform::PointerButton, prelude::*,
            typography::TextMeasurer,
        };
        let value = State::new(false);
        let state = SwitchInteractionState::new();
        let switch = Switch::with_interaction(value.binding(), state.clone());
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut list = DisplayList::new();
        let bounds = Rect::new(0.0, 0.0, 80.0, 32.0);
        switch.paint(
            bounds,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
        );
        let mark = state.drag.inner.borrow().mark_bounds.unwrap();
        let start = Point::new(mark.origin.x + 8.0, mark.origin.y + mark.size.height * 0.5);
        let end = Point::new(mark.origin.x + mark.size.width - 3.0, start.y);
        let mut context = EventContext::new(&theme, &theme.typography, &mut text);
        for event in [
            ViewEvent::PointerPressed {
                position: start,
                button: PointerButton::Primary,
            },
            ViewEvent::PointerMoved { position: end },
            ViewEvent::PointerReleased {
                position: end,
                button: PointerButton::Primary,
            },
        ] {
            switch.handle_event(bounds, &event, &mut context);
        }
        assert!(
            value.get(),
            "release must commit before any animation frames"
        );
        let rebuilt = Switch::with_interaction(value.binding(), state.clone());
        for _ in 0..4 {
            rebuilt.paint(
                bounds,
                &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
            );
            assert!(value.get());
        }
        assert!(!state.drag.inner.borrow().tracking);
    }

    #[test]
    fn default_thumb_is_slightly_wider_than_tall() {
        let metrics = SwitchMetrics::from_theme(&Theme::LIGHT);
        assert!(metrics.knob_width > metrics.knob_height);
    }

    #[test]
    fn thumb_width_is_derived_from_theme_height_and_aspect_ratio() {
        let mut theme = Theme::LIGHT;
        theme.layout.control_thumb_height = 16.0;
        theme.layout.control_thumb_aspect_ratio = 1.25;
        theme.layout.control_pressed_thumb_aspect_ratio = 1.5;

        let metrics = SwitchMetrics::from_theme(&theme);
        assert_eq!(metrics.knob_height, 16.0);
        assert_eq!(metrics.knob_width, 20.0);
        assert!(metrics.knob_width <= metrics.track_width - metrics.knob_inset * 2.0);
    }
}
