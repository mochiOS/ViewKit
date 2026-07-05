use std::cell::RefCell;
use std::rc::Rc;

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, Card, Divider, HStack, Padding, Text,
    VStack, ZStackAlignment,
};

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
    on_select: Option<Rc<RefCell<Box<dyn FnMut()>>>>,
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
            theme.colors.text_disabled
        } else if self.danger {
            theme.colors.destructive
        } else {
            theme.colors.text_primary
        };

        let shortcut_color = if self.enabled {
            theme.colors.text_tertiary
        } else {
            theme.colors.text_disabled
        };

        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(
                Text::new(self.label.clone())
                    .font_size(12.0)
                    .line_height(20.0)
                    .weight(500)
                    .color(foreground)
                    .layout()
                    .flex_grow(1.0),
            );

        if let Some(shortcut) = self.shortcut.as_ref() {
            content = content.child(
                Text::new(shortcut.clone())
                    .font_size(11.0)
                    .line_height(20.0)
                    .color(shortcut_color),
            );
        }

        let style = if self.danger {
            ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: theme.colors.destructive_soft,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.colors.destructive,
            }
        } else {
            ButtonStyle::Ghost
        };

        let mut button = Button::with_interaction(self.interaction.clone())
            .style(style)
            .radius(CornerRadius::Small)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::symmetric(10.0, 5.0).content(content));

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
        self.content = std::mem::take(&mut self.content).child(item.height(34.0));
        self
    }

    pub fn separator(mut self) -> Self {
        self.content = std::mem::take(&mut self.content).child(Divider::new());
        self
    }

    fn card(&self) -> Card<Padding<ViewRef<'_, VStack>>> {
        Card::new()
            .radius(CornerRadius::Large)
            .shadow(ShadowStyle::Card)
            .border(BorderStyle::Standard { width: 1.0 })
            .content(Padding::all(6.0).content(ViewRef::new(&self.content)))
    }
}

impl View for Menu {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.card().measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
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
