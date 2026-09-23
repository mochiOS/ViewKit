//! Shared placeholder used when an application does not provide an icon.

use crate::geometry::{Rect, Size};
use crate::theme::{Color, CornerRadius, Theme};
use crate::typography::{TextAlignment, TextRole};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor, Text};

/// Draws the same accent monogram used by Binder for applications without an
/// icon. Keeping it in ViewKit prevents system applications from inventing
/// incompatible placeholders.
pub struct ApplicationPlaceholder {
    name: String,
}

impl ApplicationPlaceholder {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    fn initial(&self) -> String {
        self.name
            .chars()
            .find(|character| character.is_alphanumeric())
            .unwrap_or('?')
            .to_uppercase()
            .collect()
    }
}

impl View for ApplicationPlaceholder {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        let side = constraints
            .maximum
            .width
            .min(constraints.maximum.height)
            .max(constraints.minimum.width.max(constraints.minimum.height));
        constraints.constrain(Size::new(side, side))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        Rectangle::new()
            .color(RectangleColor::Custom(Theme::current().colors.accent))
            .radius(CornerRadius::Custom(bounds.size.width * 0.22))
            .paint(bounds, context);
        Text::styled(self.initial(), TextRole::TitleMedium)
            .font_size(bounds.size.width * 0.48)
            .weight(600)
            .alignment(TextAlignment::Center)
            .color(Color::WHITE)
            .paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + bounds.size.height * 0.18,
                    bounds.size.width,
                    bounds.size.height * 0.7,
                ),
                context,
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_uses_the_first_alphanumeric_character() {
        assert_eq!(ApplicationPlaceholder::new("  files").initial(), "F");
        assert_eq!(ApplicationPlaceholder::new("---").initial(), "?");
    }
}
