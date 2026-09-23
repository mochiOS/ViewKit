//! Iconコンポーネント
//!
//! アイコン資産はFigmaを正として再構築中です。公開APIは維持しますが、
//! 新しい資産が登録されるまではアイコンを描画しません。

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::geometry::{Rect, Size};
use crate::svg::{SvgContentBounds, SvgData};
use crate::theme::{Color, LayoutTokens};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/symbols.rs"));

impl SymbolName {
    pub(crate) const fn control_size(self, layout: LayoutTokens) -> f32 {
        match self {
            Self::ArrowTop | Self::ArrowUp => layout.compact_icon_size,
            _ => layout.control_icon_size,
        }
    }
}

#[deprecated(since = "2.1.0", note = "use `SymbolName` instead")]
pub type IconName = SymbolName;

#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    name: SymbolName,

    size: Option<f32>,
    color: Color,
    opacity: f32,
    accessibility_label: Option<String>,
}

impl Icon {
    pub const fn new(name: SymbolName) -> Self {
        Self {
            name,

            size: None,

            color: Color::from_rgb_hex(0x17181a),

            opacity: 1.0,

            accessibility_label: None,
        }
    }

    pub const fn name(&self) -> SymbolName {
        self.name
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = sanitize_size(size);

        self
    }

    pub const fn color(mut self, color: Color) -> Self {
        self.color = color;

        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = sanitize_opacity(opacity);

        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl View for Icon {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let size = self.size.unwrap_or(context.theme.layout.icon_button_size);
        constraints.constrain(Size::new(size, size))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let Some(svg) = self.name.svg() else {
            return;
        };
        let artwork = normalized_svg_bounds(
            bounds,
            &svg,
            context.theme.layout.icon_optical_scale,
        );
        if let Some(label) = self.accessibility_label.as_ref() {
            let mut node = AccessibilityNode::new(AccessibilityRole::Image, bounds);
            node.label = Some(label.clone());
            context.record_accessibility(node);
        }
        let symbol = super::svg::Svg::new(svg)
            .tint(self.color)
            .opacity(self.opacity);
        context.display_list.push(DrawCommand::PushClip { rect: bounds });
        symbol.paint(artwork, context);
        context.display_list.push(DrawCommand::PopClip);
    }
}

fn normalized_svg_bounds(bounds: Rect, svg: &SvgData, optical_scale: f32) -> Rect {
    let content = svg.content_bounds();
    if !valid_content_bounds(content) {
        return bounds;
    }

    let optical_scale = if optical_scale.is_finite() && optical_scale > 0.0 {
        optical_scale.min(1.0)
    } else {
        1.0
    };
    let target_width = bounds.size.width * optical_scale;
    let target_height = bounds.size.height * optical_scale;
    let scale = (target_width / content.width).min(target_height / content.height);
    if !scale.is_finite() || scale <= 0.0 {
        return bounds;
    }

    let content_center_x = content.x + content.width / 2.0;
    let content_center_y = content.y + content.height / 2.0;
    let target_center_x = bounds.origin.x + bounds.size.width / 2.0;
    let target_center_y = bounds.origin.y + bounds.size.height / 2.0;
    Rect::new(
        target_center_x - content_center_x * scale,
        target_center_y - content_center_y * scale,
        svg.width() * scale,
        svg.height() * scale,
    )
}

fn valid_content_bounds(bounds: SvgContentBounds) -> bool {
    bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0
}

fn sanitize_size(size: f32) -> Option<f32> {
    if size.is_finite() && size > 0.0 {
        Some(size)
    } else {
        None
    }
}

fn sanitize_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg_with_rect(x: u32) -> SvgData {
        SvgData::decode(
            format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="{x}" y="6" width="10" height="8"/></svg>"#
            )
            .as_bytes(),
        )
        .expect("test SVG should decode")
    }

    #[test]
    fn optical_normalization_ignores_canvas_padding() {
        let bounds = Rect::new(10.0, 20.0, 20.0, 20.0);
        let left = svg_with_rect(2);
        let right = svg_with_rect(10);
        let left_bounds = normalized_svg_bounds(bounds, &left, 0.84);
        let right_bounds = normalized_svg_bounds(bounds, &right, 0.84);
        let left_content = left.content_bounds();
        let right_content = right.content_bounds();

        let left_scale = left_bounds.size.width / left.width();
        let right_scale = right_bounds.size.width / right.width();
        let left_visual_center = left_bounds.origin.x
            + (left_content.x + left_content.width / 2.0) * left_scale;
        let right_visual_center = right_bounds.origin.x
            + (right_content.x + right_content.width / 2.0) * right_scale;

        assert!((left_visual_center - 20.0).abs() < 0.001);
        assert!((right_visual_center - 20.0).abs() < 0.001);
        assert!((left_content.width * left_scale - 16.8).abs() < 0.001);
        assert!((right_content.width * right_scale - 16.8).abs() < 0.001);
    }
}
