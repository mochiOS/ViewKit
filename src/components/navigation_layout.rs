//! Standard toolbar/sidebar/detail layout from the ViewKit Layout Example.

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor, Surface};

/// Owns the standard application chrome used by sidebar-based applications.
///
/// The toolbar spans the full width. The sidebar starts below it, and the
/// toolbar/sidebar junction receives the theme's medium-radius concave corner.
/// Its position and maximum size are derived from the resolved regions, so
/// applications do not specify or paint the inner corner themselves.
pub struct NavigationLayout {
    toolbar: StackChild,
    sidebar: StackChild,
    detail: StackChild,
}

impl NavigationLayout {
    pub fn new<ToolbarContent, SidebarContent, Detail>(
        toolbar: ToolbarContent,
        sidebar: SidebarContent,
        detail: Detail,
    ) -> Self
    where
        ToolbarContent: IntoStackChild,
        SidebarContent: IntoStackChild,
        Detail: IntoStackChild,
    {
        Self {
            toolbar: toolbar.into_stack_child(),
            sidebar: sidebar.into_stack_child(),
            detail: detail.into_stack_child(),
        }
    }

    fn regions(bounds: Rect, context: &PaintContext<'_>) -> (Rect, Rect, Rect) {
        Self::resolved_regions(
            bounds,
            context.theme.layout.top_bar_height,
            context.theme.layout.navigation_sidebar_width,
            context.theme.spacing.extra_large,
            context.theme.spacing.small,
            context.theme.spacing.medium,
            context.theme.spacing.large,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolved_regions(
        bounds: Rect,
        toolbar_height: f32,
        sidebar_width: f32,
        toolbar_horizontal: f32,
        toolbar_vertical: f32,
        sidebar_horizontal: f32,
        sidebar_vertical: f32,
    ) -> (Rect, Rect, Rect) {
        let toolbar_height = toolbar_height.min(bounds.size.height).max(0.0);
        let sidebar_width = sidebar_width.min(bounds.size.width).max(0.0);
        let toolbar = Rect::new(
            bounds.origin.x + toolbar_horizontal,
            bounds.origin.y + toolbar_vertical,
            (bounds.size.width - toolbar_horizontal * 2.0).max(0.0),
            (toolbar_height - toolbar_vertical * 2.0).max(0.0),
        );
        let sidebar = Rect::new(
            bounds.origin.x + sidebar_horizontal,
            bounds.origin.y + toolbar_height + sidebar_vertical,
            (sidebar_width - sidebar_horizontal * 2.0).max(0.0),
            (bounds.size.height - toolbar_height - sidebar_vertical * 2.0).max(0.0),
        );
        let detail = Rect::new(
            bounds.origin.x + sidebar_width,
            bounds.origin.y + toolbar_height,
            (bounds.size.width - sidebar_width).max(0.0),
            (bounds.size.height - toolbar_height).max(0.0),
        );
        (toolbar, sidebar, detail)
    }
}

impl View for NavigationLayout {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let width = if constraints.maximum.width.is_finite() {
            constraints.maximum.width
        } else {
            context.theme.layout.standard_window_width
        };
        let height = if constraints.maximum.height.is_finite() {
            constraints.maximum.height
        } else {
            context.theme.layout.standard_window_height
        };
        constraints.constrain(Size::new(width, height))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.is_empty() {
            return;
        }

        Rectangle::new()
            .color(RectangleColor::SubtleSurface)
            .paint(bounds, context);

        let toolbar_height = context
            .theme
            .layout
            .top_bar_height
            .min(bounds.size.height)
            .max(0.0);
        let sidebar_width = context
            .theme
            .layout
            .navigation_sidebar_width
            .min(bounds.size.width)
            .max(0.0);
        Rectangle::new().color(RectangleColor::SubtleSurface).paint(
            Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                toolbar_height,
            ),
            context,
        );
        Rectangle::new().color(RectangleColor::SubtleSurface).paint(
            Rect::new(
                bounds.origin.x,
                bounds.origin.y + toolbar_height,
                sidebar_width,
                (bounds.size.height - toolbar_height).max(0.0),
            ),
            context,
        );

        let (toolbar, sidebar, detail) = Self::regions(bounds, context);
        context.record_accessibility(AccessibilityNode::new(AccessibilityRole::Toolbar, toolbar));
        context.record_accessibility(AccessibilityNode::new(
            AccessibilityRole::Navigation,
            sidebar,
        ));
        Surface::pane()
            .in_container(context.pane_container().unwrap_or(bounds))
            .paint_content(detail, context, |context| {
                self.detail.paint(detail, context);
            });

        self.toolbar.paint(toolbar, context);
        self.sidebar.paint(sidebar, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let (toolbar, sidebar, detail) = Self::resolved_regions(
            bounds,
            context.theme.layout.top_bar_height,
            context.theme.layout.navigation_sidebar_width,
            context.theme.spacing.extra_large,
            context.theme.spacing.small,
            context.theme.spacing.medium,
            context.theme.spacing.large,
        );
        if event.requires_broadcast() {
            return self
                .toolbar
                .handle_event(toolbar, event, context)
                .merge(self.sidebar.handle_event(sidebar, event, context))
                .merge(
                    Surface::pane()
                        .in_container(context.pane_container().unwrap_or(bounds))
                        .handle_content_event(detail, event, context, |event, context| {
                            self.detail.handle_event(detail, event, context)
                        }),
                );
        }
        if event.is_inside(toolbar) {
            self.toolbar.handle_event(toolbar, event, context)
        } else if event.is_inside(sidebar) {
            self.sidebar.handle_event(sidebar, event, context)
        } else if event.is_inside(detail) {
            Surface::pane()
                .in_container(context.pane_container().unwrap_or(bounds))
                .handle_content_event(detail, event, context, |event, context| {
                    self.detail.handle_event(detail, event, context)
                })
        } else {
            EventResult::Ignored
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::{DisplayList, DrawCommand};
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Recorder(Rc<Cell<Option<Rect>>>);

    impl View for Recorder {
        fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
            self.0.set(Some(bounds));
            context.display_list.push(DrawCommand::FillRect {
                rect: bounds,
                color: Theme::LIGHT.colors.surface,
            });
        }
    }

    #[test]
    fn regions_match_the_layout_example() {
        let toolbar = Rc::new(Cell::new(None));
        let sidebar = Rc::new(Cell::new(None));
        let detail = Rc::new(Cell::new(None));
        let layout = NavigationLayout::new(
            Recorder(toolbar.clone()),
            Recorder(sidebar.clone()),
            Recorder(detail.clone()),
        );
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        layout.paint(Rect::new(0.0, 0.0, 1040.0, 700.0), &mut context);

        assert_eq!(toolbar.get(), Some(Rect::new(24.0, 8.0, 992.0, 32.0)));
        assert_eq!(sidebar.get(), Some(Rect::new(12.0, 64.0, 216.0, 620.0)));
        assert_eq!(detail.get(), Some(Rect::new(240.0, 48.0, 800.0, 652.0)));
        let corner_index = display_list.commands().iter().position(|command| matches!(command,
            DrawCommand::PushRoundedCornersClip { rect, radii }
                if *rect == detail.get().unwrap() && *radii == [Theme::LIGHT.radius.medium, 0.0, 0.0, 0.0]
        )).expect("detail clip");
        let detail_index = display_list
            .commands()
            .iter()
            .rposition(|command| {
                matches!(command,
                    DrawCommand::FillRect { rect, .. } if *rect == detail.get().unwrap()
                )
            })
            .expect("opaque detail content");
        assert!(
            corner_index < detail_index,
            "opaque children must be inside the clip"
        );
        assert!(matches!(
            display_list.commands()[detail_index + 1],
            DrawCommand::PopClip
        ));
        assert!(
            !display_list
                .commands()
                .iter()
                .any(|command| matches!(command, DrawCommand::DrawContour { .. }))
        );
        assert!(!display_list.commands().iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, .. }
                if rect.size.width == Theme::LIGHT.divider.thickness
                    && rect.size.height == 652.0
        )));
    }

    #[test]
    fn inner_corner_uses_the_active_theme_radius() {
        let mut theme = Theme::LIGHT;
        theme.radius.medium = 7.0;
        let layout = NavigationLayout::new(
            Recorder(Rc::new(Cell::new(None))),
            Recorder(Rc::new(Cell::new(None))),
            Recorder(Rc::new(Cell::new(None))),
        );
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &theme,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        layout.paint(Rect::new(0.0, 0.0, 1040.0, 700.0), &mut context);

        assert!(display_list.commands().iter().any(|command| matches!(command,
            DrawCommand::PushRoundedCornersClip { radii, .. } if *radii == [7.0, 0.0, 0.0, 0.0]
        )));
    }
}
