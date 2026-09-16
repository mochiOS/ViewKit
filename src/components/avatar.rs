use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::geometry::{Rect, Size};
use crate::theme::Color;
use crate::typography::{TextAlignment, TextRole};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Ellipse, EllipseColor, Text};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Small,

    #[default]
    Medium,
}

impl AvatarSize {
    fn length(self, context: &MeasureContext<'_>) -> f32 {
        match self {
            Self::Small => context.theme.layout.avatar_small_size,
            Self::Medium => context.theme.layout.avatar_size,
        }
    }

    const fn text_role(self) -> TextRole {
        match self {
            Self::Small => TextRole::Caption,
            Self::Medium => TextRole::Label,
        }
    }
}

pub struct Avatar {
    initials: String,
    size: AvatarSize,
    background: Option<Color>,
    foreground: Option<Color>,
    accessibility_label: Option<String>,
}

impl Avatar {
    pub fn new(initials: impl Into<String>) -> Self {
        Self {
            initials: initials.into(),
            size: AvatarSize::Medium,
            background: None,
            foreground: None,
            accessibility_label: None,
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl View for Avatar {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let length = self.size.length(context);
        constraints.constrain(Size::new(length, length))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let mut node = AccessibilityNode::new(AccessibilityRole::Image, bounds);
        node.label = self
            .accessibility_label
            .clone()
            .or_else(|| Some(self.initials.clone()));
        context.record_accessibility(node);

        Ellipse::new()
            .color(EllipseColor::Custom(
                self.background
                    .unwrap_or(context.theme.avatar.background),
            ))
            .paint(bounds, context);

        let line_height = context
            .typography
            .style(self.size.text_role())
            .line_height
            .min(bounds.size.height);
        let text_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y + (bounds.size.height - line_height) / 2.0,
            bounds.size.width,
            line_height,
        );

        Text::styled(self.initials.clone(), self.size.text_role())
            .accessibility_hidden(true)
            .alignment(TextAlignment::Center)
            .color(
                self.foreground
                    .unwrap_or(context.theme.avatar.foreground),
            )
            .paint(text_bounds, context);
    }
}
