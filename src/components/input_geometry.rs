//! Shared text line geometry for native input controls.
use crate::{geometry::Rect, theme::ControlSize, typography::TextRole};

pub(super) const fn input_text_role(size: ControlSize) -> TextRole {
    match size {
        ControlSize::Small => TextRole::Caption,
        ControlSize::Medium => TextRole::Label,
        ControlSize::Large => TextRole::Body,
    }
}

pub(super) fn field_text_line(
    bounds: Rect,
    style: crate::typography::TextStyle,
    scale: f32,
    offset: f32,
) -> Rect {
    let font_size = super::text::resolved_font_size(style.size) * scale;
    let height = super::text::resolved_line_height(font_size, style.line_height * scale);
    Rect::new(
        bounds.origin.x,
        bounds.origin.y + (bounds.size.height - height) * 0.5 + offset,
        bounds.size.width,
        height,
    )
}
