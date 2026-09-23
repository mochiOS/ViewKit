use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap};
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Icon, Rectangle, RectangleColor,
    SymbolName, Text, VStack, ZStackAlignment,
};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

/// A text-only navigation item using the selection treatment from the Layout
/// Example. Icons are intentionally not synthesized; callers may add only
/// symbols that exist in VK Symbols through a custom control.
pub struct SidebarItem {
    label: String,
    symbol: Option<SymbolName>,
    selected: bool,
    interaction: ButtonInteractionState,
    on_select: Option<Callback>,
}

impl SidebarItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            symbol: None,
            selected: false,
            interaction: ButtonInteractionState::new(),
            on_select: None,
        }
    }

    pub fn symbol(mut self, symbol: SymbolName) -> Self {
        self.symbol = Some(symbol);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_select(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_select = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }

    fn button(&self, theme: &Theme) -> Button {
        let foreground = if self.selected {
            theme.colors.text_primary
        } else {
            theme.colors.text_secondary
        };
        let style = if self.selected {
            ButtonStyle::Custom {
                background: theme.colors.accent_soft,
                hovered_background: theme.colors.accent_soft,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground,
            }
        } else {
            ButtonStyle::Ghost
        };
        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small);
        if let Some(symbol) = self.symbol {
            content = content.child(
                Icon::new(symbol)
                    .size(theme.layout.compact_icon_size)
                    .color(foreground),
            );
        }
        content = content.child(
            Text::label(self.label.clone())
                .weight(if self.selected { 600 } else { 500 })
                .color(foreground),
        );
        let mut button = Button::with_interaction(self.interaction.clone())
            .content(content)
            .style(style)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .radius(CornerRadius::Custom(6.0))
            .accessibility_role(AccessibilityRole::Button)
            .accessibility_label(self.label.clone());
        if let Some(on_select) = &self.on_select {
            let on_select = Rc::clone(on_select);
            button = button.on_click(move || (on_select.borrow_mut())());
        }
        button
    }
}

impl View for SidebarItem {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let measured = self.button(context.theme).measure(constraints, context);
        constraints.constrain(Size::new(
            measured.width,
            context.theme.layout.control_height,
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

/// A caption followed by standard sidebar navigation items.
pub struct SidebarSection {
    content: VStack,
}

impl SidebarSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            content: VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::ExtraSmall)
                .child(
                    Text::caption(title.into()).color(Theme::current().colors.text_secondary),
                ),
        }
    }

    pub fn item(mut self, item: SidebarItem) -> Self {
        self.content = std::mem::take(&mut self.content).child(item);
        self
    }
}

impl View for SidebarSection {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.content.paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.content.handle_event(bounds, event, context)
    }
}

/// A standard mochiOS navigation surface.
///
/// `Sidebar` owns the sidebar background and content insets so applications do
/// not need to reproduce those details. `NavigationSplitView` remains useful
/// when the sidebar and detail should be arranged as a desktop split view.
pub struct Sidebar<Content> {
    content: Content,
}

impl<Content> Sidebar<Content> {
    pub const fn new(content: Content) -> Self {
        Self { content }
    }

    fn content_bounds(bounds: Rect, horizontal: f32, vertical: f32) -> Rect {
        Rect::new(
            bounds.origin.x + horizontal,
            bounds.origin.y + vertical,
            (bounds.size.width - horizontal * 2.0).max(0.0),
            (bounds.size.height - vertical * 2.0).max(0.0),
        )
    }
}

impl<Content: View> View for Sidebar<Content> {
    fn stack_flex_shrink(&self) -> f32 {
        0.0
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let horizontal = context.theme.spacing.medium;
        let vertical = context.theme.spacing.large;
        let content = self.content.measure(
            Constraints::loose(Size::new(
                (context.theme.layout.navigation_sidebar_width - horizontal * 2.0).max(0.0),
                (constraints.maximum.height - vertical * 2.0).max(0.0),
            )),
            context,
        );

        constraints.constrain(Size::new(
            context.theme.layout.navigation_sidebar_width,
            content.height + vertical * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.record_accessibility(AccessibilityNode::new(
            AccessibilityRole::Navigation,
            bounds,
        ));
        Rectangle::new()
            .color(RectangleColor::SubtleSurface)
            .paint(bounds, context);
        self.content.paint(
            Self::content_bounds(
                bounds,
                context.theme.spacing.medium,
                context.theme.spacing.large,
            ),
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.content.handle_event(
            Self::content_bounds(
                bounds,
                context.theme.spacing.medium,
                context.theme.spacing.large,
            ),
            event,
            context,
        )
    }
}
