//! Viewの背面へ背景を配置するBackgroundを定義

use crate::geometry::Rect;
use crate::view::{
    PaintContext,
    View,
};

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyView;

impl View for EmptyView {
    fn paint(
        &self,
        _bounds: Rect,
        _context: &mut PaintContext<'_>,
    ) {
    }
}

pub struct Background<
    B = EmptyView,
    C = EmptyView,
> {
    background: B,
    content: C,
}

impl Background<EmptyView, EmptyView> {
    pub const fn new() -> Self {
        Self {
            background: EmptyView,
            content: EmptyView,
        }
    }
}

impl Default for Background<EmptyView, EmptyView> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, C> Background<B, C> {
    pub fn background<NewBackground>(
        self,
        background: NewBackground,
    ) -> Background<NewBackground, C>
    where
        NewBackground: View,
    {
        Background {
            background,
            content: self.content,
        }
    }

    pub fn content<NewContent>(
        self,
        content: NewContent,
    ) -> Background<B, NewContent>
    where
        NewContent: View,
    {
        Background {
            background: self.background,
            content,
        }
    }
}

impl<B, C> View for Background<B, C>
where
    B: View,
    C: View,
{
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        self.background.paint(
            bounds,
            context,
        );

        self.content.paint(
            bounds,
            context,
        );
    }
}