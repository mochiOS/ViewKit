use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChild, StackAlignment, StackGap, ViewExt};
use crate::theme::{Color, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, Card, Divider, HStack, Padding,
    Spacer, Text, VStack, ZStackAlignment,
};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

struct ViewRef<'a, V>
where
    V: View + ?Sized,
{
    view: &'a V,
}

impl<'a, V> ViewRef<'a, V>
where
    V: View + ?Sized,
{
    fn new(view: &'a V) -> Self {
        Self { view }
    }
}

impl<V> View for ViewRef<'_, V>
where
    V: View + ?Sized,
{
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.view.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.view.paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.view.handle_event(bounds, event, context)
    }
}

pub struct MenuItem {
    label: String,
    shortcut: Option<String>,

    enabled: bool,
    danger: bool,

    interaction: ButtonInteractionState,
    on_select: Option<Callback>,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,

            enabled: true,
            danger: false,

            interaction: ButtonInteractionState::new(),
            on_select: None,
        }
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn on_select(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_select = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    fn button(&self, theme: &Theme) -> Button {
        let foreground = if !self.enabled {
            theme.menu.disabled_foreground
        } else if self.danger {
            theme.menu.danger_foreground
        } else {
            theme.menu.foreground
        };

        let shortcut_color = if self.enabled {
            theme.menu.secondary_foreground
        } else {
            theme.menu.disabled_foreground
        };

        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(
                Text::label(self.label.clone())
                    .accessibility_hidden(true)
                    .color(foreground)
                    .layout()
                    .flex_grow(1.0),
            );

        if let Some(shortcut) = self.shortcut.as_ref() {
            content = content.child(
                Text::caption(shortcut.clone())
                    .accessibility_hidden(true)
                    .color(shortcut_color),
            );
        }

        let style = if self.danger {
            ButtonStyle::Custom {
                background: theme.menu.item_background,
                hovered_background: theme.menu.danger_hovered_background,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.menu.danger_foreground,
            }
        } else {
            ButtonStyle::Custom {
                background: theme.menu.item_background,
                hovered_background: theme.menu.item_hovered_background,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.menu.foreground,
            }
        };

        let mut button = Button::with_interaction(self.interaction.clone())
            .style(style)
            .radius(theme.menu.item_radius)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .accessibility_role(AccessibilityRole::MenuItem)
            .accessibility_label(self.label.clone())
            .accessibility_value_option(self.shortcut.clone())
            .content(
                Padding::symmetric(
                    theme.menu.item_horizontal_padding,
                    theme.menu.item_vertical_padding,
                )
                .content(content),
            );

        if let Some(on_select) = self.on_select.as_ref() {
            let on_select = Rc::clone(on_select);

            button = button.on_click(move || {
                (on_select.borrow_mut())();
            });
        }

        button
    }
}

impl View for MenuItem {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.button(context.theme).measure(constraints, context)
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
        self.button(context.theme)
            .handle_event(bounds, event, context)
    }
}

pub struct Menu {
    content: VStack,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            content: VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None),
        }
    }
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.content = std::mem::take(&mut self.content)
            .child(item.height(Theme::current().layout.compact_control_height));
        self
    }

    pub fn separator(mut self) -> Self {
        let spacing = Theme::current().spacing.extra_small;
        self.content = std::mem::take(&mut self.content)
            .child(Spacer::new().into_stack_child().height(spacing))
            .child(Divider::new())
            .child(Spacer::new().into_stack_child().height(spacing));

        self
    }

    fn card(&self) -> Card<ViewRef<'_, VStack>> {
        Card::new()
            .compact()
            .radius(Theme::current().menu.surface_radius)
            .shadow(ShadowStyle::Card)
            .border(BorderStyle::Standard {
                width: Theme::current().menu.surface_stroke_width,
            })
            .content(ViewRef::new(&self.content))
    }
}

impl View for Menu {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.card().measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.record_accessibility(AccessibilityNode::new(AccessibilityRole::Menu, bounds));
        self.card().paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.card().handle_event(bounds, event, context)
    }
}
