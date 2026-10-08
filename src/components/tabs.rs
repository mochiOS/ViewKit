use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::animation::{Animation, interpolate};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{Color, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::time::Instant;

use super::omochi_shape::{OmochiPreset, OmochiShape};
use super::{Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Text, ZStackAlignment};

struct TabItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

pub struct Tabs {
    selection: Binding<usize>,
    items: Vec<TabItem>,
    enabled: bool,
    accessibility_label: Option<String>,
    indicator_shape: OmochiShape,
}

impl Tabs {
    pub fn new(selection: Binding<usize>) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
            indicator_shape: OmochiShape::velocity(OmochiPreset::SelectionIndicator),
        }
    }

    pub fn item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(TabItem {
            value,
            label: label.into(),
            enabled: true,
            interaction: ButtonInteractionState::new(),
        });
        self
    }

    pub fn disabled_item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(TabItem {
            value,
            label: label.into(),
            enabled: false,
            interaction: ButtonInteractionState::new(),
        });
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    fn item_button(&self, item: &TabItem, theme: &Theme) -> Button {
        let selected = self.selection.get() == item.value;
        let enabled = self.enabled && item.enabled;
        let selection = self.selection.clone();
        let value = item.value;

        let foreground = if !enabled {
            theme.tabs.disabled_foreground
        } else if selected {
            theme.tabs.selected_foreground
        } else {
            theme.tabs.foreground
        };

        let background = theme.tabs.background;

        Button::with_interaction(item.interaction.clone())
            .style(ButtonStyle::Custom {
                background,
                hovered_background: if selected {
                    theme.tabs.background
                } else {
                    theme.tabs.hovered_background
                },
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground,
            })
            .radius(theme.tabs.radius)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Center)
            .enabled(enabled)
            .accessibility_role(AccessibilityRole::Tab)
            .accessibility_label(item.label.clone())
            .accessibility_selected(selected)
            .content(
                Padding::symmetric(theme.tabs.horizontal_padding, theme.tabs.vertical_padding)
                    .content(
                        Text::label(item.label.clone())
                            .accessibility_hidden(true)
                            .color(foreground),
                    ),
            )
            .on_click(move || {
                selection.set_if_changed(value);
            })
    }

    fn stack(&self, theme: &Theme) -> HStack {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .children(self.items.iter().map(|item| {
                self.item_button(item, theme)
                    .frame(theme.layout.tab_width, theme.layout.compact_control_height)
            }))
    }

    fn selected_index(&self, value: usize) -> Option<usize> {
        self.items.iter().position(|item| item.value == value)
    }

    fn animated_index(&self, now: Instant, theme: &Theme) -> (Option<f32>, Option<Instant>) {
        let value = self.selection.get();
        let Some(current) = self.selected_index(value) else {
            return (None, None);
        };
        let Some(transition) = self.selection.transition() else {
            return (Some(current as f32), None);
        };
        let (Some(from), Some(to)) = (
            self.selected_index(transition.from),
            self.selected_index(transition.to),
        ) else {
            return (Some(current as f32), None);
        };
        if transition.to != value || from == to {
            return (Some(current as f32), None);
        }
        let sample = Animation::new(transition.started_at, theme.motion.selection.duration)
            .easing(theme.motion.selection.easing)
            .sample(now);
        (
            Some(interpolate(from as f32, to as f32, sample.progress)),
            sample.next_redraw_at,
        )
    }

    fn keyboard_target(&self, event: &ViewEvent) -> Option<usize> {
        if !self.enabled || self.items.is_empty() {
            return None;
        }
        let current = self
            .items
            .iter()
            .position(|item| item.interaction.is_focused())?;
        let direction = match event {
            ViewEvent::KeyPressed {
                key: Key::ArrowRight | Key::ArrowDown,
                ..
            } => 1,
            ViewEvent::KeyPressed {
                key: Key::ArrowLeft | Key::ArrowUp,
                ..
            } => -1,
            ViewEvent::KeyPressed { key: Key::Home, .. } => {
                return self.items.iter().position(|item| item.enabled);
            }
            ViewEvent::KeyPressed { key: Key::End, .. } => {
                return self.items.iter().rposition(|item| item.enabled);
            }
            _ => return None,
        };
        let mut index = current;
        for _ in 0..self.items.len() {
            index = if direction > 0 {
                (index + 1) % self.items.len()
            } else {
                (index + self.items.len() - 1) % self.items.len()
            };
            if self.items[index].enabled {
                return Some(index);
            }
        }
        None
    }
}

impl View for Tabs {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.stack(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let mut node = AccessibilityNode::new(AccessibilityRole::TabList, bounds);
        node.label = self.accessibility_label.clone();
        node.enabled = self.enabled;
        context.record_accessibility(node);
        if !self.enabled && self.indicator_shape.is_animating() {
            self.indicator_shape.reset();
        }
        let now = Instant::now();
        let (index, next_redraw) = self.animated_index(now, context.theme);
        if let Some(next_redraw) = next_redraw {
            context.request_redraw_in_at(bounds.expanded(16.0), next_redraw);
        }
        if let Some(index) = index {
            let gap = StackGap::Small.resolve(&context.theme.spacing);
            let width = context.theme.layout.tab_width;
            let indicator = Rect::new(
                bounds.origin.x + (width + gap) * index,
                bounds.origin.y,
                width,
                context
                    .theme
                    .layout
                    .compact_control_height
                    .min(bounds.size.height),
            );
            let radius = context.theme.tabs.radius.resolve(
                &context.theme.radius,
                indicator.size.width,
                indicator.size.height,
            );
            context
                .display_list
                .push(DrawCommand::PushClip { rect: bounds });
            self.indicator_shape.paint(
                indicator,
                radius,
                context.theme.tabs.selected_background,
                context,
            );
            context.display_list.push(DrawCommand::PopClip);
        }
        self.stack(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if self.enabled && bounds.contains(*position) => {
                self.indicator_shape.begin(*position)
            }
            ViewEvent::PointerMoved { position } if self.indicator_shape.is_active() => {
                self.indicator_shape.moved(*position);
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } if self.indicator_shape.is_active() => {
                self.indicator_shape.moved(*position);
                self.indicator_shape.end();
            }
            ViewEvent::FocusChanged { focused: false } if self.indicator_shape.is_active() => {
                self.indicator_shape.end();
            }
            _ => {}
        }
        let stack = self.stack(context.theme);
        if let Some(index) = self.keyboard_target(event) {
            let child_bounds = stack.child_bounds_for_event(bounds, context);
            if let Some(target_bounds) = child_bounds.get(index).copied() {
                self.selection.set_if_changed(self.items[index].value);
                context.request_keyboard_focus(target_bounds);
                context.request_redraw_in(bounds.expanded(16.0));
                return EventResult::Consumed;
            }
        }
        stack.handle_event(bounds, event, context)
    }
}
