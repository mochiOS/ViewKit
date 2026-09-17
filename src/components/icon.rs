//! Iconコンポーネント
//!
//! アイコン資産はFigmaを正として再構築中です。公開APIは維持しますが、
//! 新しい資産が登録されるまではアイコンを描画しません。

use crate::geometry::Size;
use crate::svg::SvgData;
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

    fn paint(&self, bounds: crate::geometry::Rect, context: &mut PaintContext<'_>) {
        let Some(svg) = self.name.svg() else {
            return;
        };
        let mut symbol = super::svg::Svg::new(svg)
            .tint(self.color)
            .opacity(self.opacity);
        if let Some(label) = self.accessibility_label.as_ref() {
            symbol = symbol.accessibility_label(label.clone());
        }
        symbol.paint(bounds, context);
    }
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
