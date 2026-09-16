use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::platform::Key;
use crate::state::Binding;
use crate::theme::{Color, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

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
}

impl Tabs {
    pub fn new(selection: Binding<usize>) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
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

        let background = if selected {
            theme.tabs.selected_background
        } else {
            theme.tabs.background
        };

        Button::with_interaction(item.interaction.clone())
            .style(ButtonStyle::Custom {
                background,
                hovered_background: if selected {
                    theme.tabs.selected_background
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
        self.stack(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
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
