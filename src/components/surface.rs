//! Standard page and panel surfaces.

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::theme::Theme;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::background::EmptyView;
use super::{BorderStyle, Rectangle, RectangleColor};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurfaceKind {
    #[default]
    App,
    Pane,
    Sidebar,
    Floating,
}

pub struct Surface<Content = EmptyView> {
    kind: SurfaceKind,
    content: Content,
}

impl Surface<EmptyView> {
    pub const fn new(kind: SurfaceKind) -> Self {
        Self {
            kind,
            content: EmptyView,
        }
    }

    pub const fn app() -> Self {
        Self::new(SurfaceKind::App)
    }

    pub const fn pane() -> Self {
        Self::new(SurfaceKind::Pane)
    }

    pub const fn sidebar() -> Self {
        Self::new(SurfaceKind::Sidebar)
    }

    pub const fn floating() -> Self {
        Self::new(SurfaceKind::Floating)
    }
}

impl<Content> Surface<Content> {
    pub fn content<NewContent>(self, content: NewContent) -> Surface<NewContent>
    where
        NewContent: View,
    {
        Surface {
            kind: self.kind,
            content,
        }
    }

    fn chrome(&self, theme: &Theme) -> Rectangle {
        match self.kind {
            SurfaceKind::App => Rectangle::new().color(RectangleColor::Custom(
                theme.surface.app_background,
            )),
            SurfaceKind::Pane => Rectangle::new().color(RectangleColor::Custom(
                theme.surface.pane_background,
            )),
            SurfaceKind::Sidebar => Rectangle::new().color(RectangleColor::Custom(
                theme.surface.sidebar_background,
            )),
            SurfaceKind::Floating => Rectangle::new()
                .color(RectangleColor::Custom(theme.surface.floating_background))
                .radius(theme.surface.floating_radius)
                .shadow(theme.surface.floating_shadow)
                .border(BorderStyle::custom(
                    theme.surface.floating_border,
                    theme.surface.floating_stroke_width,
                )),
        }
    }
}

impl Default for Surface<EmptyView> {
    fn default() -> Self {
        Self::app()
    }
}

impl<Content> View for Surface<Content>
where
    Content: View,
{
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.chrome(context.theme).paint(bounds, context);
        self.content.paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.content.handle_event(bounds, event, context)
    }
}
