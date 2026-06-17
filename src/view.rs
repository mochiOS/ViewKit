//! Viewの共通インターフェースを定義

use crate::draw_command::DisplayList;
use crate::geometry::Rect;
use crate::theme::Theme;
use crate::typography::Typography;

pub struct PaintContext<'a> {
    pub display_list: &'a mut DisplayList,
    pub theme: &'a Theme,
    pub typography: &'a Typography,
}

pub trait View {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    );
}