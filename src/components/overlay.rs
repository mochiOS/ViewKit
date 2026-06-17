//! Viewの前面へ別のViewを重ねるOverlayを定義

use crate::geometry::Rect;
use crate::view::{
    PaintContext,
    View,
};

#[derive(Default)]
pub struct Overlay {
    content: Option<Box<dyn View>>,
    overlay: Option<Box<dyn View>>,
}

impl Overlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content<V>(
        mut self,
        content: V,
    ) -> Self
    where
        V: View + 'static,
    {
        self.content = Some(Box::new(content));
        self
    }

    pub fn overlay<V>(
        mut self,
        overlay: V,
    ) -> Self
    where
        V: View + 'static,
    {
        self.overlay = Some(Box::new(overlay));
        self
    }
}

impl View for Overlay {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        if let Some(content) = &self.content {
            content.paint(
                bounds,
                context,
            );
        }

        if let Some(overlay) = &self.overlay {
            overlay.paint(
                bounds,
                context,
            );
        }
    }
}