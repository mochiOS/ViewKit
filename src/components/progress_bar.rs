use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

pub struct ProgressBar {
    value: f32,
    minimum: f32,
    maximum: f32,
    enabled: bool,
    accessibility_label: Option<String>,
}

impl ProgressBar {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            minimum: 0.0,
            maximum: 1.0,
            enabled: true,
            accessibility_label: None,
        }
    }

    pub fn range(mut self, minimum: f32, maximum: f32) -> Self {
        if minimum.is_finite() && maximum.is_finite() && minimum != maximum {
            self.minimum = minimum.min(maximum);
            self.maximum = minimum.max(maximum);
        }
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

    fn progress(&self) -> f32 {
        let value = if self.value.is_finite() {
            self.value
        } else {
            self.minimum
        };

        ((value.clamp(self.minimum, self.maximum) - self.minimum) / (self.maximum - self.minimum))
            .clamp(0.0, 1.0)
    }
}

impl View for ProgressBar {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(
            context.theme.layout.range_control_width,
            context.theme.layout.range_control_height,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let track_height = context.theme.layout.range_track_height;
        let track = Rect::new(
            bounds.origin.x,
            bounds.origin.y + (bounds.size.height - track_height) / 2.0,
            bounds.size.width,
            track_height,
        );

        let tokens = context.theme.progress_bar;
        let track_color = if self.enabled {
            tokens.track
        } else {
            tokens.disabled_track
        };

        let fill_color = if self.enabled {
            tokens.fill
        } else {
            tokens.disabled_fill
        };

        let mut node = AccessibilityNode::new(AccessibilityRole::ProgressIndicator, bounds);
        node.label = self.accessibility_label.clone();
        node.numeric_value = Some(
            self.minimum + self.progress() * (self.maximum - self.minimum),
        );
        node.numeric_minimum = Some(self.minimum);
        node.numeric_maximum = Some(self.maximum);
        node.enabled = self.enabled;
        context.record_accessibility(node);

        Rectangle::new()
            .color(RectangleColor::Custom(track_color))
            .radius(tokens.radius)
            .paint(track, context);

        let fill_width = track.size.width * self.progress();
        if fill_width > 0.0 {
            Rectangle::new()
                .color(RectangleColor::Custom(fill_color))
                .radius(tokens.radius)
                .paint(
                    Rect::new(
                        track.origin.x,
                        track.origin.y,
                        fill_width,
                        track.size.height,
                    ),
                    context,
                );
        }
    }
}
