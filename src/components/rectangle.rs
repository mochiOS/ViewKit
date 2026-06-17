//! 矩形コンポーネントを定義

use crate::draw_command::DrawCommand;
use crate::geometry::Rect;
use crate::theme::{
    Color,
    CornerRadius,
    Shadow,
    ShadowStyle,
};
use crate::view::{
    PaintContext,
    View,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RectangleColor {
    Background,
    Surface,
    ElevatedSurface,
    Accent,
    Destructive,
    Custom(Color),
}

impl RectangleColor {
    fn resolve(
        self,
        context: &PaintContext<'_>,
    ) -> Color {
        match self {
            Self::Background => {
                context.theme.colors.background
            }

            Self::Surface => {
                context.theme.colors.surface
            }

            Self::ElevatedSurface => {
                context.theme.colors.elevated_surface
            }

            Self::Accent => {
                context.theme.colors.accent
            }

            Self::Destructive => {
                context.theme.colors.destructive
            }

            Self::Custom(color) => color,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    color: RectangleColor,
    radius: CornerRadius,
    shadow: ShadowStyle,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            color: RectangleColor::Surface,
            radius: CornerRadius::Medium,
            shadow: ShadowStyle::Small,
        }
    }
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(
        mut self,
        color: RectangleColor,
    ) -> Self {
        self.color = color;
        self
    }

    pub fn radius(
        mut self,
        radius: CornerRadius,
    ) -> Self {
        self.radius = radius;
        self
    }

    pub fn shadow(
        mut self,
        shadow: ShadowStyle,
    ) -> Self {
        self.shadow = shadow;
        self
    }
}

impl View for Rectangle {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        let color = self.color.resolve(context);

        let radius = self.radius.resolve(
            &context.theme.radius,
            bounds.size.width,
            bounds.size.height,
        );

        if let Some(shadow) = self
            .shadow
            .resolve(&context.theme.shadows)
        {
            paint_shadow(
                bounds,
                radius,
                shadow,
                context,
            );
        }

        if radius > 0.0 {
            context.display_list.push(
                DrawCommand::FillRoundedRect {
                    rect: bounds,
                    radius,
                    color,
                },
            );
        } else {
            context.display_list.push(
                DrawCommand::FillRect {
                    rect: bounds,
                    color,
                },
            );
        }
    }
}

fn paint_shadow(
    bounds: Rect,
    radius: f32,
    shadow: Shadow,
    context: &mut PaintContext<'_>,
) {
    if shadow.color.alpha == 0 {
        return;
    }

    let blur_radius =
        shadow.blur_radius.max(0.0);

    let spread =
        shadow.spread.max(0.0);

    if blur_radius == 0.0 {
        let shadow_bounds = expanded_shadow_rect(
            bounds,
            shadow.offset_x,
            shadow.offset_y,
            spread,
        );

        context.display_list.push(
            DrawCommand::FillRoundedRect {
                rect: shadow_bounds,
                radius: radius + spread,
                color: shadow.color,
            },
        );

        return;
    }

    let layers = blur_radius
        .ceil()
        .clamp(2.0, 24.0) as u32;

    for layer in (1..=layers).rev() {
        let progress =
            layer as f32 / layers as f32;

        let expansion =
            spread + blur_radius * progress;

        let opacity_weight =
            1.0 - progress * 0.75;

        let alpha = (
            shadow.color.alpha as f32
                * opacity_weight
                * 2.0
                / layers as f32
        )
            .round()
            .clamp(1.0, 255.0) as u8;

        let shadow_bounds = expanded_shadow_rect(
            bounds,
            shadow.offset_x,
            shadow.offset_y,
            expansion,
        );

        context.display_list.push(
            DrawCommand::FillRoundedRect {
                rect: shadow_bounds,
                radius: radius + expansion,
                color: shadow
                    .color
                    .with_alpha(alpha),
            },
        );
    }
}

fn expanded_shadow_rect(
    bounds: Rect,
    offset_x: f32,
    offset_y: f32,
    expansion: f32,
) -> Rect {
    Rect::new(
        bounds.origin.x
            + offset_x
            - expansion,
        bounds.origin.y
            + offset_y
            - expansion,
        bounds.size.width
            + expansion * 2.0,
        bounds.size.height
            + expansion * 2.0,
    )
}