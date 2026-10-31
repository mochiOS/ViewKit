//! 矩形コンポーネントを定義

use crate::draw_command::DrawCommand;
use crate::geometry::{Rect, Size};
use crate::theme::{Color, CornerRadius, Shadow, ShadowSet, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RectangleColor {
    Background,
    Surface,
    SubtleSurface,
    ElevatedSurface,
    Accent,
    Destructive,
    Custom(Color),
}

impl RectangleColor {
    pub(crate) fn resolve(self, context: &PaintContext<'_>) -> Color {
        match self {
            Self::Background => context.theme.colors.background,
            Self::Surface => context.theme.colors.surface,
            Self::SubtleSurface => context.theme.colors.surface_subtle,
            Self::ElevatedSurface => context.theme.colors.elevated_surface,
            Self::Accent => context.theme.colors.accent,
            Self::Destructive => context.theme.colors.destructive,
            Self::Custom(color) => color,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BorderStyle {
    #[default]
    None,

    Standard {
        width: f32,
    },

    Strong {
        width: f32,
    },

    Custom {
        color: Color,
        width: f32,
    },
}

impl BorderStyle {
    pub const fn standard(width: f32) -> Self {
        Self::Standard { width }
    }

    pub const fn strong(width: f32) -> Self {
        Self::Strong { width }
    }

    pub const fn custom(color: Color, width: f32) -> Self {
        Self::Custom { color, width }
    }

    pub(crate) fn resolve(self, context: &PaintContext<'_>) -> Option<(Color, f32)> {
        let (color, width) = match self {
            Self::None => {
                return None;
            }

            Self::Standard { width } => (context.theme.colors.border, width),

            Self::Strong { width } => (context.theme.colors.border_strong, width),

            Self::Custom { color, width } => (color, width),
        };

        if !width.is_finite() || width <= 0.0 || color.alpha == 0 {
            return None;
        }

        Some((color, width))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    color: RectangleColor,
    radius: CornerRadius,
    shadow: ShadowStyle,
    border: BorderStyle,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            color: RectangleColor::Surface,
            radius: CornerRadius::None,
            shadow: ShadowStyle::None,
            border: BorderStyle::None,
        }
    }
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
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

impl View for Rectangle {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::ZERO)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let color = self.color.resolve(context);

        let radius =
            self.radius
                .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);

        if let Some(shadow_set) = self.shadow.resolve(&context.theme.shadows) {
            paint_shadow_set(bounds, radius, shadow_set, context);
        }

        // A transparent fill has no visible contribution, including on GPU backends.
        if color.alpha != 0 {
            if radius > 0.0 {
                context.display_list.push(DrawCommand::FillRoundedRect {
                    rect: bounds,
                    radius,
                    color,
                });
            } else {
                context.display_list.push(DrawCommand::FillRect {
                    rect: bounds,
                    color,
                });
            }
        }

        paint_border(bounds, radius, self.border, context);
    }
}

#[allow(unused)]
fn paint_border(bounds: Rect, radius: f32, border: BorderStyle, context: &mut PaintContext<'_>) {
    let Some((color, width)) = border.resolve(context) else {
        return;
    };

    if radius > 0.0 {
        context
            .display_list
            .push(DrawCommand::StrokeInsideRoundedRect {
                rect: bounds,
                radius,
                color,
                width,
            });
        return;
    }
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

    let border_radius = (radius - half_width).max(0.0);

    if border_radius > 0.0 {
        context.display_list.push(DrawCommand::StrokeRoundedRect {
            rect: border_bounds,
            radius: border_radius,
            color,
            width,
        });
    } else {
        context.display_list.push(DrawCommand::StrokeRect {
            rect: border_bounds,
            color,
            width,
        });
    }
}

fn paint_shadow_set(
    bounds: Rect,
    radius: f32,
    shadow_set: ShadowSet,
    context: &mut PaintContext<'_>,
) {
    for shadow in shadow_set.layers.iter().rev().flatten() {
        paint_shadow(bounds, radius, *shadow, context);
    }
}

fn paint_shadow(bounds: Rect, radius: f32, shadow: Shadow, context: &mut PaintContext<'_>) {
    if shadow.color.alpha == 0 {
        return;
    }

    for (expansion, color) in shadow_layers(shadow) {
        let shadow_bounds =
            expanded_shadow_rect(bounds, shadow.offset_x, shadow.offset_y, expansion);
        context.display_list.push(DrawCommand::FillRoundedRect {
            rect: shadow_bounds,
            radius: radius + expansion,
            color,
        });
    }
}

// The same bounded coverage profile is used by rigid and deforming surfaces.
// Return outer layers first so compositing and total opacity agree at rest.
pub(super) fn shadow_layers(shadow: Shadow) -> Vec<(f32, Color)> {
    if shadow.color.alpha == 0 {
        return Vec::new();
    }
    let blur = if shadow.blur_radius.is_finite() {
        shadow.blur_radius.max(0.0)
    } else {
        0.0
    };
    let spread = if shadow.spread.is_finite() {
        shadow.spread.max(0.0)
    } else {
        0.0
    };
    if blur == 0.0 {
        return vec![(spread, shadow.color)];
    }
    let layers = blur
        .ceil()
        .clamp(2.0, 32.0)
        .min(f32::from(shadow.color.alpha).max(1.0)) as u32;
    let weight_sum = (1..=layers)
        .map(|i| 1.0 - (i as f32 / layers as f32) * 0.75)
        .sum::<f32>();
    let mut remaining = u32::from(shadow.color.alpha);
    let mut result = Vec::with_capacity(layers as usize);
    for layer in (1..=layers).rev() {
        let progress = layer as f32 / layers as f32;
        let mut alpha =
            (f32::from(shadow.color.alpha) * (1.0 - progress * 0.75) / weight_sum).round() as u32;
        alpha = alpha.min(remaining);
        if layer == 1 {
            alpha = remaining;
        }
        remaining -= alpha;
        if alpha > 0 {
            result.push((
                spread + blur * progress,
                shadow.color.with_alpha(alpha as u8),
            ));
        }
    }
    result
}

fn expanded_shadow_rect(bounds: Rect, offset_x: f32, offset_y: f32, expansion: f32) -> Rect {
    Rect::new(
        bounds.origin.x + offset_x - expansion,
        bounds.origin.y + offset_y - expansion,
        bounds.size.width + expansion * 2.0,
        bounds.size.height + expansion * 2.0,
    )
}
