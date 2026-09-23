use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::accessibility::AccessibilityRole;
use crate::event::{
    ContextMenuItem, ContextMenuRequest, EventContext, EventResult, ViewEvent,
};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::platform::PointerButton;
use crate::theme::{CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Icon, Padding, Spacer, SymbolName, Text,
    ZStackAlignment,
};

pub struct Picker {
    label: String,
    enabled: bool,
    radius: Option<CornerRadius>,
    interaction: ButtonInteractionState,
    options: Vec<PickerOption>,
    active_request: Cell<Option<u64>>,
}

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

struct PickerOption {
    label: String,
    enabled: bool,
    on_select: Callback,
}

static NEXT_PICKER_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

impl Picker {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            radius: None,
            interaction: ButtonInteractionState::new(),
            options: Vec::new(),
            active_request: Cell::new(None),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Adds a selectable value to the picker menu.
    pub fn option(
        mut self,
        label: impl Into<String>,
        on_select: impl FnMut() + 'static,
    ) -> Self {
        self.options.push(PickerOption {
            label: label.into(),
            enabled: true,
            on_select: Rc::new(RefCell::new(Box::new(on_select))),
        });
        self
    }

    /// Adds a disabled value while preserving menu ordering.
    pub fn disabled_option(mut self, label: impl Into<String>) -> Self {
        self.options.push(PickerOption {
            label: label.into(),
            enabled: false,
            on_select: Rc::new(RefCell::new(Box::new(|| {}))),
        });
        self
    }

    fn open_menu(&self, bounds: Rect, context: &mut EventContext<'_>) {
        if self.options.is_empty() {
            return;
        }
        let request_id = NEXT_PICKER_REQUEST_ID
            .fetch_add(1, Ordering::Relaxed)
            .max(1);
        self.active_request.set(Some(request_id));
        context.show_context_menu(ContextMenuRequest {
            request_id,
            position: Point::new(bounds.origin.x, bounds.origin.y + bounds.size.height),
            items: self
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| ContextMenuItem {
                    command_id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    label: option.label.clone(),
                    enabled: option.enabled,
                    checked: option.label == self.label,
                    destructive: false,
                    separator: false,
                })
                .collect(),
        });
    }

    fn button(&self, theme: &Theme) -> Button {
        let foreground = if self.enabled {
            theme.picker.foreground
        } else {
            theme.picker.disabled_foreground
        };

        Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: theme.picker.background,
                hovered_background: theme.picker.hovered_background,
                border: theme.picker.border,
                hovered_border: theme.picker.hovered_border,
                foreground,
            })
            .radius(self.radius.unwrap_or(theme.picker.radius))
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .accessibility_role(AccessibilityRole::ComboBox)
            .accessibility_label(self.label.clone())
            .content(
                Padding::symmetric(
                    theme.picker.horizontal_padding,
                    theme.picker.vertical_padding,
                )
                .content(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Small)
                        .child(
                            Text::label(self.label.clone())
                                .accessibility_hidden(true)
                                .color(foreground)
                                .layout()
                                .flex_grow(1.0),
                        )
                        .child(Spacer::new())
                        .child(
                            Icon::new(SymbolName::ChevronDown)
                                .size(theme.layout.compact_icon_size)
                                .color(theme.picker.indicator)
                                .frame(
                                    theme.layout.compact_icon_size,
                                    theme.layout.compact_icon_size,
                                ),
                        ),
                ),
            )
    }
}

impl View for Picker {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(self.button(context.theme).measure(
            Constraints::new(
                Size::new(
                    context.theme.layout.control_min_width,
                    context.theme.layout.compact_control_height,
                ),
                Size::new(f32::INFINITY, context.theme.layout.compact_control_height),
            ),
            context,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.button(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if let ViewEvent::ContextMenuResult {
            request_id,
            command_id,
        } = event
            && self.active_request.get() == Some(*request_id)
        {
            self.active_request.set(None);
            if let Some(index) = command_id.and_then(|id| id.checked_sub(1))
                && let Some(option) = self.options.get(index as usize)
                && option.enabled
            {
                (option.on_select.borrow_mut())();
                context.request_redraw();
            }
            return EventResult::Consumed;
        }

        let result = self
            .button(context.theme)
            .handle_event(bounds, event, context);
        if self.enabled
            && result.is_consumed()
            && matches!(
                event,
                ViewEvent::PointerReleased {
                    position,
                    button: PointerButton::Primary,
                } if bounds.contains(*position)
            )
        {
            self.open_menu(bounds, context);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn options_open_a_menu_and_dispatch_the_selected_callback() {
        let selected = Rc::new(Cell::new(false));
        let callback_state = Rc::clone(&selected);
        let picker = Picker::new("Japanese").option("English", move || {
            callback_state.set(true);
        });
        let bounds = Rect::new(20.0, 30.0, 264.0, 32.0);
        let position = Point::new(40.0, 45.0);
        let mut text_measurer = TextMeasurer::new();
        let mut context = EventContext::new(
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        picker.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        picker.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            },
            &mut context,
        );

        let request = context.take_context_menu_request().expect("picker menu");
        assert_eq!(request.position, Point::new(20.0, 62.0));
        assert_eq!(request.items.len(), 1);
        assert_eq!(request.items[0].label, "English");
        assert!(!request.items[0].checked);

        picker.handle_event(
            bounds,
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id,
                command_id: Some(1),
            },
            &mut context,
        );
        assert!(selected.get());
    }
}
