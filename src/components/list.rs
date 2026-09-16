//! リストコンポーネント

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{
    IntoStackChildren, StackAlignment, StackChild, StackDirection, StackDistribution, StackGap,
    ViewExt, handle_stack_event, measure_stack, paint_stack,
};
use crate::theme::{Color, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use super::{
    Avatar, Button, ButtonInteractionState, ButtonStyle, Ellipse, EllipseColor, HStack, Icon,
    IconName, Padding, Text, VStack, ZStackAlignment,
};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

/// A standard edge-to-edge collection of list rows.
pub struct List {
    children: Vec<StackChild>,
}

impl List {
    pub const fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn row<Row: IntoStackChildren>(mut self, row: Row) -> Self {
        self.children.extend(row.into_stack_children());
        self
    }

    pub fn rows<Row: IntoStackChildren>(
        mut self,
        rows: impl IntoIterator<Item = Row>,
    ) -> Self {
        for row in rows {
            self.children.extend(row.into_stack_children());
        }
        self
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl View for List {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        measure_stack(
            StackDirection::Vertical,
            &self.children,
            StackGap::None,
            constraints,
            context,
        )
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.record_accessibility(AccessibilityNode::new(AccessibilityRole::List, bounds));
        paint_stack(
            StackDirection::Vertical,
            &self.children,
            bounds,
            StackGap::None,
            StackAlignment::Stretch,
            StackDistribution::Start,
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        handle_stack_event(
            StackDirection::Vertical,
            &self.children,
            bounds,
            StackGap::None,
            StackAlignment::Stretch,
            StackDistribution::Start,
            event,
            context,
        )
    }
}

pub struct ListRow {
    title: String,
    subtitle: Option<String>,
    trailing: Option<String>,

    selected: bool,
    enabled: bool,

    interaction: ButtonInteractionState,
    on_select: Option<Callback>,
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
        let mut title_row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                Text::label(self.title.clone())
                    .accessibility_hidden(true)
                    .color(if self.selected {
                        theme.list.selected_foreground
                    } else {
                        theme.list.foreground
                    })
                    .layout()
                    .flex_grow(1.0),
            );

        if let Some(trailing) = self.trailing.as_ref() {
            title_row =
                title_row.child(
                    Text::caption(trailing.clone()).color(theme.list.secondary_foreground),
                );
        }

        let mut labels = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::ExtraSmall)
            .child(title_row);

        if self.subtitle.is_some() || self.status_marker {
            let mut subtitle_row = HStack::new()
                .alignment(StackAlignment::Center)
                .gap(StackGap::Small);

            if let Some(subtitle) = self.subtitle.as_ref() {
                subtitle_row = subtitle_row.child(
                    Text::caption(subtitle.clone())
                        .color(theme.list.secondary_foreground)
                        .layout()
                        .flex_grow(1.0),
                );
            } else {
                subtitle_row = subtitle_row.child(super::Spacer::new());
            }

            if self.status_marker {
                subtitle_row = subtitle_row.child(
                    Ellipse::new()
                        .color(EllipseColor::Custom(theme.list.status_marker))
                        .frame(
                            theme.layout.status_marker_size,
                            theme.layout.status_marker_size,
                        ),
                );
            }

            labels = labels.child(subtitle_row);
        }

        let mut row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium);

        if let Some(initials) = self.leading_avatar.as_ref() {
            row = row.child(Avatar::new(initials.clone()));
        } else if let Some(icon) = self.icon {
            row = row.child(
                Icon::new(icon)
                    .size(theme.layout.compact_icon_size)
                    .color(theme.list.leading_foreground)
                    .frame(
                        theme.layout.list_leading_size,
                        theme.layout.list_leading_size,
                    ),
            );
        }

        row = row.child(labels.layout().flex_grow(1.0));

        Padding::all(theme.list.row_padding).content(row)
    }

    fn button(&self, theme: &Theme) -> Button {
        let mut button = Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: if self.selected {
                    theme.list.selected_background
                } else {
                    theme.list.background
                },
                hovered_background: if self.selected {
                    theme.list.selected_background
                } else {
                    theme.list.hovered_background
                },
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.list.foreground,
            })
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(self.content_view(theme));

        button = button
            .accessibility_role(AccessibilityRole::ListItem)
            .accessibility_label(self.title.clone())
            .accessibility_selected(self.selected);

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
        let measured = self.button(context.theme).measure(constraints, context);
        constraints.constrain(Size::new(
            measured.width,
            measured.height.max(context.theme.layout.list_row_height),
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
