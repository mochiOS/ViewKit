//! リストコンポーネント

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::theme::{Color, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;

use super::{
    Avatar, Button, ButtonInteractionState, ButtonStyle, Ellipse, EllipseColor, HStack, Icon,
    IconName, Padding, Text, VStack, ZStackAlignment,
};

pub struct ListRow {
    title: String,
    subtitle: Option<String>,
    trailing: Option<String>,

    selected: bool,
    enabled: bool,

    interaction: ButtonInteractionState,
    on_select: Option<Rc<RefCell<Box<dyn FnMut()>>>>,
    icon: Option<IconName>,
    leading_avatar: Option<String>,
    status_marker: bool,
}

impl ListRow {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            trailing: None,

            selected: false,
            enabled: true,

            interaction: ButtonInteractionState::new(),
            on_select: None,
            icon: None,
            leading_avatar: None,
            status_marker: false,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn trailing(mut self, trailing: impl Into<String>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self.leading_avatar = None;
        self
    }

    pub fn leading_avatar(mut self, initials: impl Into<String>) -> Self {
        self.leading_avatar = Some(initials.into());
        self.icon = None;
        self
    }

    pub fn status_marker(mut self, visible: bool) -> Self {
        self.status_marker = visible;
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

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    fn content_view(&self, theme: &Theme) -> Padding<HStack> {
        let mut labels = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(
                Text::new(self.title.clone())
                    .font_size(13.0)
                    .line_height(18.0)
                    .weight(500)
                    .color(if self.selected {
                        theme.colors.accent
                    } else {
                        theme.colors.text_primary
                    }),
            );

        if let Some(subtitle) = self.subtitle.as_ref() {
            labels = labels.child(
                Text::new(subtitle.clone())
                    .font_size(11.0)
                    .line_height(16.0)
                    .color(theme.colors.text_secondary),
            );
        }

        let mut row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small);

        if let Some(initials) = self.leading_avatar.as_ref() {
            row = row.child(Avatar::new(initials.clone()).frame(32.0, 32.0));
        } else if let Some(icon) = self.icon {
            row = row.child(
                Icon::new(icon)
                    .size(14.0)
                    .color(theme.colors.text_secondary)
                    .frame(24.0, 24.0),
            );
        }

        row = row.child(labels.layout().flex_grow(1.0));

        if let Some(trailing) = self.trailing.as_ref() {
            row = row.child(
                Text::new(trailing.clone())
                    .font_size(11.0)
                    .line_height(16.0)
                    .color(theme.colors.text_secondary),
            );
        }

        if self.status_marker {
            row = row.child(
                Ellipse::new()
                    .color(EllipseColor::Custom(theme.colors.accent))
                    .frame(8.0, 8.0),
            );
        }

        Padding::symmetric(12.0, 8.0).content(row)
    }

    fn button(&self, theme: &Theme) -> Button {
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
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(self.content_view(theme));

        if let Some(on_select) = self.on_select.as_ref() {
            let on_select = Rc::clone(on_select);
            button = button.on_click(move || {
                (on_select.borrow_mut())();
            });
        }

        button
    }
}

impl View for ListRow {
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
