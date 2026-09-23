use crate::draw_command::DrawCommand;
use crate::edge_insets::EdgeInsets;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::theme::Theme;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

pub struct NavigationSplitView<Sidebar, Detail> {
    sidebar: Sidebar,
    detail: Detail,
    sidebar_width: SidebarWidth,
    minimum_detail_width: f32,
    sidebar_insets: Option<EdgeInsets>,
    shows_divider: bool,
}

/// Width policy for the leading region of a [`NavigationSplitView`].
///
/// `Flexible` is the default. It preserves the theme's preferred sidebar
/// width when space permits, but yields space to the detail region as the
/// window becomes narrower. Applications which truly require a fixed-width
/// inspector can opt into `Fixed` explicitly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SidebarWidth {
    Flexible {
        minimum: f32,
        ideal: Option<f32>,
        maximum: f32,
    },
    Fixed(f32),
}

impl Default for SidebarWidth {
    fn default() -> Self {
        Self::Flexible {
            minimum: 160.0,
            ideal: None,
            maximum: 320.0,
        }
    }
}

impl<Sidebar, Detail> NavigationSplitView<Sidebar, Detail> {
    pub fn new(sidebar: Sidebar, detail: Detail) -> Self {
        Self {
            sidebar,
            detail,
            sidebar_width: SidebarWidth::default(),
            minimum_detail_width: 320.0,
            sidebar_insets: None,
            shows_divider: true,
        }
    }

    /// Applies an explicit leading-region sizing policy.
    ///
    /// This is the general entry point for reusable components which expose
    /// their own layout configuration. The convenience methods below cover
    /// the common fixed and three-value flexible cases.
    pub fn sidebar_width(mut self, width: SidebarWidth) -> Self {
        self.sidebar_width = width.sanitized();
        self
    }

    /// Uses a fixed leading-region width. Prefer [`Self::flexible_sidebar`]
    /// for resizable application windows.
    pub fn fixed_sidebar(self, width: f32) -> Self {
        self.sidebar_width(SidebarWidth::Fixed(width))
    }

    /// Constrains the leading region while allowing it to adapt to the
    /// available window width.
    pub fn flexible_sidebar(self, minimum: f32, ideal: f32, maximum: f32) -> Self {
        self.sidebar_width(SidebarWidth::Flexible {
            minimum,
            ideal: Some(ideal),
            maximum,
        })
    }

    /// Reserves this much width for the detail region before shrinking the
    /// sidebar. The value is a constraint, not an absolute detail width.
    pub fn minimum_detail_width(mut self, width: f32) -> Self {
        self.minimum_detail_width = sanitize(width);
        self
    }

    /// Overrides the standard sidebar content insets.
    pub fn sidebar_insets(mut self, insets: EdgeInsets) -> Self {
        self.sidebar_insets = Some(insets.sanitized());
        self
    }

    pub fn shows_divider(mut self, shows_divider: bool) -> Self {
        self.shows_divider = shows_divider;
        self
    }

    fn resolved_insets(&self, theme: &Theme) -> EdgeInsets {
        self.sidebar_insets.unwrap_or_else(|| {
            EdgeInsets::symmetric(theme.spacing.medium, theme.spacing.large)
        })
    }

    fn resolved_divider_width(&self, theme: &Theme) -> f32 {
        if self.shows_divider {
            sanitize(theme.divider.thickness)
        } else {
            0.0
        }
    }

    fn resolved_sidebar_width(&self, available_width: f32, theme: &Theme) -> f32 {
        let available_width = sanitize(available_width);
        let divider = self.resolved_divider_width(theme).min(available_width);
        let room_before_detail =
            (available_width - divider - self.minimum_detail_width).max(0.0);
        match self.sidebar_width {
            SidebarWidth::Fixed(width) => width.min((available_width - divider).max(0.0)),
            SidebarWidth::Flexible {
                minimum,
                ideal,
                maximum,
            } => {
                let minimum = sanitize(minimum);
                let maximum = sanitize(maximum).max(minimum);
                let ideal = ideal
                    .unwrap_or(theme.layout.navigation_sidebar_width)
                    .clamp(minimum, maximum);
                ideal.min(room_before_detail.max(minimum).min(available_width - divider))
            }
        }
    }

    fn regions(bounds: Rect, sidebar_width: f32, divider_width: f32) -> (Rect, Rect, Rect) {
        let sidebar_width = sidebar_width.min(bounds.size.width).max(0.0);
        let divider_width = divider_width
            .min((bounds.size.width - sidebar_width).max(0.0))
            .max(0.0);
        let sidebar = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            sidebar_width,
            bounds.size.height,
        );
        let divider = Rect::new(
            bounds.origin.x + sidebar_width,
            bounds.origin.y,
            divider_width,
            bounds.size.height,
        );
        let detail = Rect::new(
            divider.origin.x + divider_width,
            bounds.origin.y,
            (bounds.size.width - sidebar_width - divider_width).max(0.0),
            bounds.size.height,
        );
        (sidebar, divider, detail)
    }

    fn sidebar_content(bounds: Rect, insets: EdgeInsets) -> Rect {
        Rect::new(
            bounds.origin.x + insets.left,
            bounds.origin.y + insets.top,
            (bounds.size.width - insets.horizontal()).max(0.0),
            (bounds.size.height - insets.vertical()).max(0.0),
        )
    }
}

impl SidebarWidth {
    fn sanitized(self) -> Self {
        match self {
            Self::Fixed(width) => Self::Fixed(sanitize(width)),
            Self::Flexible {
                minimum,
                ideal,
                maximum,
            } => {
                let minimum = sanitize(minimum);
                Self::Flexible {
                    minimum,
                    ideal: ideal.filter(|value| value.is_finite()).map(sanitize),
                    maximum: sanitize(maximum).max(minimum),
                }
            }
        }
    }
}

impl<Sidebar: View, Detail: View> View for NavigationSplitView<Sidebar, Detail> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let available_width = if constraints.maximum.width.is_finite() {
            constraints.maximum.width
        } else {
            context.theme.layout.standard_window_width
        };
        let sidebar_width = self.resolved_sidebar_width(available_width, context.theme);
        let divider_width = self.resolved_divider_width(context.theme);
        let insets = self.resolved_insets(context.theme);
        let sidebar = self.sidebar.measure(
            Constraints::loose(Size::new(
                (sidebar_width - insets.horizontal()).max(0.0),
                (constraints.maximum.height - insets.vertical()).max(0.0),
            )),
            context,
        );
        let detail = self.detail.measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - sidebar_width - divider_width).max(0.0),
                constraints.maximum.height,
            )),
            context,
        );
        constraints.constrain(Size::new(
            sidebar_width + divider_width + detail.width,
            (sidebar.height + insets.vertical()).max(detail.height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let (sidebar, divider, detail) = Self::regions(
            bounds,
            self.resolved_sidebar_width(bounds.size.width, context.theme),
            self.resolved_divider_width(context.theme),
        );
        Rectangle::new()
            .color(RectangleColor::SubtleSurface)
            .paint(sidebar, context);
        if self.shows_divider {
            context.display_list.push(DrawCommand::FillRect {
                rect: divider,
                color: context.theme.colors.surface_muted,
            });
        }
        self.sidebar.paint(
            Self::sidebar_content(sidebar, self.resolved_insets(context.theme)),
            context,
        );
        self.detail.paint(detail, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let (sidebar, _, detail) = Self::regions(
            bounds,
            self.resolved_sidebar_width(bounds.size.width, context.theme),
            self.resolved_divider_width(context.theme),
        );
        let sidebar = Self::sidebar_content(sidebar, self.resolved_insets(context.theme));

        if event.requires_broadcast() {
            return self
                .sidebar
                .handle_event(sidebar, event, context)
                .merge(self.detail.handle_event(detail, event, context));
        }
        if event.is_inside(sidebar) {
            self.sidebar.handle_event(sidebar, event, context)
        } else if event.is_inside(detail) {
            self.detail.handle_event(detail, event, context)
        } else {
            EventResult::Ignored
        }
    }
}

fn sanitize(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Recorder(Rc<Cell<Option<Rect>>>);

    impl View for Recorder {
        fn paint(&self, bounds: Rect, _context: &mut PaintContext<'_>) {
            self.0.set(Some(bounds));
        }
    }

    #[test]
    fn desktop_regions_match_the_chat_screen() {
        let sidebar = Rc::new(Cell::new(None));
        let detail = Rc::new(Cell::new(None));
        let view = NavigationSplitView::new(Recorder(sidebar.clone()), Recorder(detail.clone()));
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        view.paint(Rect::new(0.0, 0.0, 1180.0, 760.0), &mut context);

        assert_eq!(sidebar.get(), Some(Rect::new(12.0, 16.0, 216.0, 728.0)));
        assert_eq!(detail.get(), Some(Rect::new(241.0, 0.0, 939.0, 760.0)));
    }

    #[test]
    fn sidebar_yields_space_to_the_detail_region() {
        let sidebar = Rc::new(Cell::new(None));
        let detail = Rc::new(Cell::new(None));
        let view = NavigationSplitView::new(Recorder(sidebar.clone()), Recorder(detail.clone()))
            .flexible_sidebar(160.0, 240.0, 320.0)
            .minimum_detail_width(420.0);
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        view.paint(Rect::new(0.0, 0.0, 600.0, 500.0), &mut context);

        assert_eq!(sidebar.get(), Some(Rect::new(12.0, 16.0, 155.0, 468.0)));
        assert_eq!(detail.get(), Some(Rect::new(180.0, 0.0, 420.0, 500.0)));
    }

    #[test]
    fn split_regions_can_omit_divider_and_use_custom_insets() {
        let sidebar = Rc::new(Cell::new(None));
        let detail = Rc::new(Cell::new(None));
        let view = NavigationSplitView::new(Recorder(sidebar.clone()), Recorder(detail.clone()))
            .fixed_sidebar(200.0)
            .sidebar_insets(EdgeInsets::new(8.0, 10.0, 12.0, 14.0))
            .shows_divider(false);
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        view.paint(Rect::new(0.0, 0.0, 800.0, 500.0), &mut context);

        assert_eq!(sidebar.get(), Some(Rect::new(14.0, 8.0, 176.0, 480.0)));
        assert_eq!(detail.get(), Some(Rect::new(200.0, 0.0, 600.0, 500.0)));
        assert!(!display_list.commands().iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, .. } if rect.size.width == Theme::LIGHT.divider.thickness
        )));
    }
}
