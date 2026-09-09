use std::cell::RefCell;
use std::rc::Rc;

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Avatar, Button, ButtonInteractionState, ButtonStyle, Ellipse, EllipseColor, HStack, Padding,
    Text, VStack, ZStackAlignment,
};

pub struct ConversationRow {
    initials: String,
    title: String,
    preview: String,
    time: Option<String>,
    unread: bool,
    selected: bool,
    enabled: bool,
    interaction: ButtonInteractionState,
    on_select: Option<Rc<RefCell<Box<dyn FnMut()>>>>,
}

impl ConversationRow {
    pub fn new(initials: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            initials: initials.into(),
            title: title.into(),
            preview: String::new(),
            time: None,
            unread: false,
            selected: false,
            enabled: true,
            interaction: ButtonInteractionState::new(),
            on_select: None,
        }
    }

    pub fn preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = preview.into();
        self
    }

    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.time = Some(time.into());
        self
    }

    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn on_select(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_select = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }

    fn button(&self, theme: &Theme) -> Button {
        let title_color = if !self.enabled {
            theme.colors.text_disabled
        } else if self.selected {
            theme.colors.accent
        } else {
            theme.colors.text_primary
        };

        let metadata_color = if self.enabled {
            theme.colors.text_secondary
        } else {
            theme.colors.text_disabled
        };

        let meta = HStack::new()
            .alignment(StackAlignment::Start)
            .gap(StackGap::Small)
            .child(
                Text::new(self.title.clone())
                    .font_size(13.0)
                    .line_height(18.0)
                    .weight(500)
                    .color(title_color)
                    .layout()
                    .flex_grow(1.0),
            )
            .child(
                Text::new(self.time.clone().unwrap_or_default())
                    .font_size(12.0)
                    .line_height(16.0)
                    .color(metadata_color),
            );

        let mut preview = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                Text::new(self.preview.clone())
                    .font_size(12.0)
                    .line_height(16.0)
                    .color(metadata_color)
                    .layout()
                    .flex_grow(1.0),
            );

        if self.unread {
            preview = preview.child(
                Ellipse::new()
                    .color(EllipseColor::Custom(theme.colors.accent))
                    .frame(8.0, 8.0),
            );
        }

        let content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(Avatar::new(self.initials.clone()).frame(32.0, 32.0))
            .child(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::ExtraSmall)
                    .child(meta.height(18.0))
                    .child(preview.height(16.0))
                    .layout()
                    .flex_grow(1.0),
            );

        let mut button = Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: if self.selected {
                    theme.colors.accent_soft
                } else {
                    Color::TRANSPARENT
                },
                hovered_background: if self.selected {
                    theme.colors.accent_soft
                } else {
                    theme.colors.surface_subtle
                },
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.colors.text_primary,
            })
            .radius(CornerRadius::Small)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::all(8.0).content(content));

        if let Some(on_select) = self.on_select.as_ref() {
            let on_select = Rc::clone(on_select);
            button = button.on_click(move || {
                (on_select.borrow_mut())();
            });
        }

        button
    }
}

impl View for ConversationRow {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(self.button(context.theme).measure(
            Constraints::new(Size::new(0.0, 64.0), Size::new(f32::INFINITY, 64.0)),
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
        self.button(context.theme)
            .handle_event(bounds, event, context)
    }
}
