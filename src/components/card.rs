//! 標準的なカードコンポーネントを定義

use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::theme::{CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::background::EmptyView;
use super::{BorderStyle, Rectangle, RectangleColor};

pub struct Card<Content = EmptyView> {
    content: Content,
    color: RectangleColor,
    radius: CornerRadius,
    shadow: ShadowStyle,
    border: BorderStyle,
}

impl Card<EmptyView> {
    pub const fn new() -> Self {
        Self {
            content: EmptyView,
            color: RectangleColor::Surface,
            radius: CornerRadius::Card,
            shadow: ShadowStyle::Card,
            border: BorderStyle::Standard { width: 1.0 },
        }
    }
}

impl Default for Card<EmptyView> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Content> Card<Content> {
    pub fn content<NewContent>(self, content: NewContent) -> Card<NewContent>
    where
        NewContent: View,
    {
        Card {
            content,
            color: self.color,
            radius: self.radius,
            shadow: self.shadow,
            border: self.border,
        }
    }

    pub fn color(mut self, color: RectangleColor) -> Self {
        self.color = color;
        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = radius;
        self
    }

    pub fn shadow(mut self, shadow: ShadowStyle) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn border(mut self, border: BorderStyle) -> Self {
        self.border = border;
        self
    }
}

impl<Content> View for Card<Content>
where
    Content: View,
{
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let resolved_radius =
            self.radius
                .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);

        Rectangle::new()
            .color(self.color)
            .radius(self.radius)
            .shadow(self.shadow)
            .border(self.border)
            .paint(bounds, context);

        context
            .display_list
            .push(DrawCommand::PushClip { rect: bounds });

        context.push_corner_radius(resolved_radius);
        self.content.paint(bounds, context);
        context.pop_corner_radius();

        context.display_list.push(DrawCommand::PopClip);
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
