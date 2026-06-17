//! ui_layoutとViewKitの座標型を接続する

use crate::geometry::{
    Point,
    Rect,
};

use ui_layout::LayoutNode;

/// LayoutNodeの最初のborder boxをViewKitのRectへ変換します。
///
/// blockまたはflexノードで使用することを想定しています。
pub fn border_box(
    node: &LayoutNode,
    parent_origin: Point,
) -> Option<Rect> {
    let box_model = node
        .layout_boxes
        .iter()
        .next()?;

    let rect = box_model.border_box;

    Some(Rect::new(
        parent_origin.x + rect.x,
        parent_origin.y + rect.y,
        rect.width,
        rect.height,
    ))
}