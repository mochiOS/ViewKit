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
    container: Option<Rect>,
}

impl Surface<EmptyView> {
    pub const fn new(kind: SurfaceKind) -> Self {
        Self {
            kind,
            content: EmptyView,
            container: None,
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
            container: self.container,
        }
    }

    /// Declares the enclosing application chrome for a manually laid out pane.
    /// Corner selection is derived from the two rectangles, using the theme's
    /// existing extra-large radius. No layout space or content inset is added.
    pub fn in_container(mut self, bounds: Rect) -> Self {
        self.container = Some(bounds);
        self
    }

    fn radii(&self, bounds: Rect, container: Option<Rect>, theme: &Theme) -> [f32; 4] {
        match self.kind {
            SurfaceKind::Pane => pane_radii(
                bounds,
                self.container.or(container),
                theme.radius.extra_large,
            ),
            SurfaceKind::Floating => {
                [theme.surface.floating_radius.resolve(
                    &theme.radius,
                    bounds.size.width,
                    bounds.size.height,
                ); 4]
            }
            _ => [0.0; 4],
        }
    }

    /// Paints custom pane contents through the same shape as composed surfaces.
    /// The clip includes child backgrounds, borders and scroll indicators.
    pub fn paint_content(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
        paint: impl FnOnce(&mut PaintContext<'_>),
    ) {
        let radii = self.radii(bounds, context.pane_container(), context.theme);
        if self.kind == SurfaceKind::Floating {
            self.chrome(context.theme).paint(bounds, context);
        }
        let clips = self.kind == SurfaceKind::Floating || radii.iter().any(|radius| *radius > 0.0);
        if clips {
            context
                .display_list
                .push(crate::draw_command::DrawCommand::PushRoundedCornersClip {
                    rect: bounds,
                    radii,
                });
        }
        if self.kind != SurfaceKind::Floating {
            self.chrome(context.theme).paint(bounds, context);
        }
        let previous = if self.kind == SurfaceKind::App {
            context.set_pane_container(Some(bounds))
        } else {
            context.pane_container()
        };
        paint(context);
        if self.kind == SurfaceKind::App {
            context.set_pane_container(previous);
        }
        if clips {
            context
                .display_list
                .push(crate::draw_command::DrawCommand::PopClip);
        }
    }

    /// Routes input through the pane's painted shape. Cleanup and keyboard
    /// events still reach children; cutouts cannot activate rectangular controls.
    pub fn handle_content_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
        handle: impl FnOnce(&ViewEvent, &mut EventContext<'_>) -> EventResult,
    ) -> EventResult {
        let radii = self.radii(bounds, context.pane_container(), context.theme);
        let outside = event
            .position()
            .is_some_and(|p| !crate::continuous_corner::shape(bounds, radii).contains((p.x, p.y)));
        let cutout = outside && event.position().is_some_and(|p| bounds.contains(p));
        let event = if cutout
            && matches!(
                event,
                ViewEvent::PointerMoved { .. } | ViewEvent::PointerReleased { .. }
            ) {
            &ViewEvent::PointerLeft
        } else {
            if outside
                && (!event.requires_broadcast()
                    || matches!(event, ViewEvent::PointerFocusRequested { .. }))
            {
                return EventResult::Ignored;
            }
            event
        };
        let previous = if self.kind == SurfaceKind::App {
            context.set_pane_container(Some(bounds))
        } else {
            context.pane_container()
        };
        let result = handle(event, context);
        if self.kind == SurfaceKind::App {
            context.set_pane_container(previous);
        }
        result
    }

    fn chrome(&self, theme: &Theme) -> Rectangle {
        match self.kind {
            SurfaceKind::App => {
                Rectangle::new().color(RectangleColor::Custom(theme.surface.app_background))
            }
            SurfaceKind::Pane => {
                Rectangle::new().color(RectangleColor::Custom(theme.surface.pane_background))
            }
            SurfaceKind::Sidebar => {
                Rectangle::new().color(RectangleColor::Custom(theme.surface.sidebar_background))
            }
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
        self.paint_content(bounds, context, |context| {
            self.content.paint(bounds, context)
        });
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.handle_content_event(bounds, event, context, |event, context| {
            self.content.handle_event(bounds, event, context)
        })
    }
}

/// Only an intersection of two interior chrome edges creates a pane corner.
/// Edges coincident with the outer surface remain square; the window owns them.
fn pane_radii(bounds: Rect, container: Option<Rect>, radius: f32) -> [f32; 4] {
    let Some(container) = container else {
        return [0.0; 4];
    };
    if bounds.is_empty() || container.is_empty() {
        return [0.0; 4];
    }
    let left = bounds.origin.x > container.origin.x;
    let top = bounds.origin.y > container.origin.y;
    let right = bounds.origin.x + bounds.size.width < container.origin.x + container.size.width;
    let bottom = bounds.origin.y + bounds.size.height < container.origin.y + container.size.height;
    crate::continuous_corner::shape(
        bounds,
        [top && left, top && right, bottom && right, bottom && left]
            .map(|round| if round { radius } else { 0.0 }),
    )
    .radii
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::{DisplayList, DrawCommand};
    use crate::geometry::Point;
    use crate::platform::PointerButton;
    use crate::typography::{TextMeasurer, Typography};
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn corners_follow_chrome_junctions_not_window_edges() {
        let app = Rect::new(10.0, 20.0, 800.0, 600.0);
        assert_eq!(pane_radii(app, Some(app), 12.0), [0.0; 4]);
        assert_eq!(
            pane_radii(Rect::new(210.0, 68.0, 600.0, 552.0), Some(app), 12.0),
            [12.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            pane_radii(Rect::new(210.0, 68.0, 600.0, 520.0), Some(app), 12.0),
            [12.0, 0.0, 0.0, 12.0]
        );
        assert_eq!(
            pane_radii(Rect::new(10.0, 68.0, 600.0, 520.0), Some(app), 12.0),
            [0.0, 12.0, 12.0, 0.0]
        );
        assert_eq!(
            pane_radii(Rect::new(210.0, 68.0, 300.0, 520.0), Some(app), 12.0),
            [12.0; 4]
        );
        assert_eq!(pane_radii(app, None, 12.0), [0.0; 4]);
    }

    #[test]
    fn small_panes_use_shared_proportional_normalization() {
        let app = Rect::new(0.0, 0.0, 100.0, 100.0);
        let pane = Rect::new(20.0, 20.0, 8.0, 4.0);
        assert_eq!(pane_radii(pane, Some(app), 12.0), [2.0; 4]);
        assert_eq!(pane_radii(pane, Some(app), f32::NAN), [0.0; 4]);
        assert_eq!(pane_radii(pane, Some(app), -1.0), [0.0; 4]);
    }

    #[test]
    fn cutout_rejects_press_scroll_focus_and_cancels_an_armed_button() {
        let app = Rect::new(0.0, 0.0, 200.0, 160.0);
        let pane = Rect::new(40.0, 40.0, 160.0, 120.0);
        let corner = Point::new(40.1, 40.1);
        let inside = Point::new(100.0, 80.0);
        let clicked = Rc::new(Cell::new(false));
        let flag = clicked.clone();
        let surface = Surface::pane()
            .in_container(app)
            .content(super::super::Button::new("Test").on_click(move || flag.set(true)));
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut measurer);
        assert_eq!(
            surface.handle_event(
                pane,
                &ViewEvent::PointerPressed {
                    position: corner,
                    button: PointerButton::Primary
                },
                &mut context
            ),
            EventResult::Ignored
        );
        assert_eq!(
            surface.handle_event(
                pane,
                &ViewEvent::PointerFocusRequested { position: corner },
                &mut context
            ),
            EventResult::Ignored
        );
        let delivered = Cell::new(false);
        surface.handle_content_event(
            pane,
            &ViewEvent::Scroll {
                position: corner,
                delta_x: 0.0,
                delta_y: 1.0,
            },
            &mut context,
            |_, _| {
                delivered.set(true);
                EventResult::Consumed
            },
        );
        assert!(!delivered.get());
        surface.handle_event(
            pane,
            &ViewEvent::PointerPressed {
                position: inside,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        surface.handle_event(
            pane,
            &ViewEvent::PointerReleased {
                position: corner,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert!(!clicked.get());
        surface.handle_event(
            pane,
            &ViewEvent::PointerReleased {
                position: inside,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert!(
            !clicked.get(),
            "release in a cutout must clear the armed press"
        );
        surface.handle_event(
            pane,
            &ViewEvent::PointerPressed {
                position: inside,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        surface.handle_event(
            pane,
            &ViewEvent::PointerReleased {
                position: inside,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert!(clicked.get());
    }

    #[test]
    fn resizing_and_collapsing_chrome_recomputes_only_the_shape() {
        let app = Rect::new(0.0, 0.0, 800.0, 600.0);
        let surface = Surface::pane().in_container(app);
        assert_eq!(
            surface.radii(Rect::new(200.0, 48.0, 600.0, 520.0), None, &Theme::LIGHT),
            [
                Theme::LIGHT.radius.extra_large,
                0.0,
                0.0,
                Theme::LIGHT.radius.extra_large
            ]
        );
        assert_eq!(
            surface.radii(Rect::new(0.0, 48.0, 800.0, 552.0), None, &Theme::LIGHT),
            [0.0; 4]
        );
        let mut theme = Theme::DARK;
        theme.radius.extra_large = 7.0;
        assert_eq!(
            surface.radii(Rect::new(200.0, 48.0, 600.0, 552.0), None, &theme),
            [7.0, 0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn app_scope_reaches_nested_split_and_restores_for_siblings() {
        let app = Rect::new(0.0, 0.0, 800.0, 600.0);
        let split = Rect::new(0.0, 48.0, 800.0, 520.0);
        let mut list = DisplayList::new();
        let mut measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut measurer,
        );
        Surface::app().paint_content(app, &mut context, |context| {
            super::super::NavigationSplitView::new(EmptyView, EmptyView).paint(split, context);
        });
        assert_eq!(context.pane_container(), None);
        assert!(list.commands().iter().any(|cmd| matches!(cmd, DrawCommand::PushRoundedCornersClip {rect, radii} if rect.origin.y == 48.0 && *radii == [Theme::LIGHT.radius.extra_large, 0.0, 0.0, Theme::LIGHT.radius.extra_large])));
        let mut context = EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut measurer);
        Surface::app().handle_content_event(
            app,
            &ViewEvent::PointerLeft,
            &mut context,
            |_, context| {
                assert_eq!(context.pane_container(), Some(app));
                EventResult::Consumed
            },
        );
        assert_eq!(context.pane_container(), None);
    }
}
