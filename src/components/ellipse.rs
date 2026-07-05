//! 楕円コンポーネント

use crate::draw_command::DrawCommand;
use crate::geometry::{Rect, Size};
use crate::theme::{Shadow, ShadowSet, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{BorderStyle, RectangleColor};

pub type EllipseColor = RectangleColor;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipse {
    color: EllipseColor,
    shadow: ShadowStyle,
    border: BorderStyle,
}

impl Default for Ellipse {
    fn default() -> Self {
        Self {
            color: EllipseColor::Surface,
            shadow: ShadowStyle::None,
            border: BorderStyle::None,
        }
    }
}

impl Ellipse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: EllipseColor) -> Self {
        self.color = color;
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

impl View for Ellipse {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::ZERO)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        if let Some(shadow_set) = self.shadow.resolve(&context.theme.shadows) {
            paint_shadow_set(bounds, shadow_set, context);
        }

        context.display_list.push(DrawCommand::FillEllipse {
            rect: bounds,
            color: self.color.resolve(context),
        });

        let Some((color, width)) = self.border.resolve(context) else {
            return;
        };

        let half_width = width / 2.0;

        let border_bounds = Rect::new(
            bounds.origin.x + half_width,
            bounds.origin.y + half_width,
            (bounds.size.width - width).max(0.0),
            (bounds.size.height - width).max(0.0),
        );

        if border_bounds.size.width <= 0.0 || border_bounds.size.height <= 0.0 {
            return;
        }

        context.display_list.push(DrawCommand::StrokeEllipse {
            rect: border_bounds,
            color,
            width,
        });
    }
}

fn paint_shadow_set(bounds: Rect, shadow_set: ShadowSet, context: &mut PaintContext<'_>) {
    for shadow in shadow_set.layers.iter().rev().flatten() {
        paint_shadow(bounds, *shadow, context);
    }
}

fn paint_shadow(bounds: Rect, shadow: Shadow, context: &mut PaintContext<'_>) {
    if shadow.color.alpha == 0 {
        return;
    }

    let blur_radius = shadow.blur_radius.max(0.0);

    let spread = shadow.spread.max(0.0);

    if blur_radius == 0.0 {
        context.display_list.push(DrawCommand::FillEllipse {
            rect: expanded_shadow_rect(bounds, shadow.offset_x, shadow.offset_y, spread),
            color: shadow.color,
        });

        return;
    }

    let layers = blur_radius.ceil().clamp(2.0, shadow.color.alpha as f32) as u32;

    let weight_sum = (1..=layers)
        .map(|layer| {
            let progress = layer as f32 / layers as f32;

            1.0 - progress * 0.75
        })
        .sum::<f32>();

    let mut remaining_alpha = u32::from(shadow.color.alpha);

    for layer in (1..=layers).rev() {
        let progress = layer as f32 / layers as f32;

        let expansion = spread + blur_radius * progress;

        let weight = 1.0 - progress * 0.75;

        let mut alpha = (shadow.color.alpha as f32 * weight / weight_sum).round() as u32;

        alpha = alpha.min(remaining_alpha);

        if layer == 1 {
            alpha = remaining_alpha;
        }

        remaining_alpha = remaining_alpha.saturating_sub(alpha);

        if alpha == 0 {
            continue;
        }

        context.display_list.push(DrawCommand::FillEllipse {
            rect: expanded_shadow_rect(bounds, shadow.offset_x, shadow.offset_y, expansion),
            color: shadow.color.with_alpha(alpha as u8),
        });
    }
}

fn expanded_shadow_rect(bounds: Rect, offset_x: f32, offset_y: f32, expansion: f32) -> Rect {
    Rect::new(
        bounds.origin.x + offset_x - expansion,
        bounds.origin.y + offset_y - expansion,
        bounds.size.width + expansion * 2.0,
        bounds.size.height + expansion * 2.0,
    )
}
