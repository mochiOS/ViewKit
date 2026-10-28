use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarPlacement {
    #[default]
    Top,
    Bottom,
}

pub struct Toolbar<Content> {
    placement: ToolbarPlacement,
    content: Content,
}

impl<Content> Toolbar<Content> {
    pub fn new(content: Content) -> Self {
        Self {
            placement: ToolbarPlacement::Top,
            content,
        }
    }

    pub fn bottom(content: Content) -> Self {
        Self {
            placement: ToolbarPlacement::Bottom,
            content,
        }
    }

    fn target_height(&self, context: &MeasureContext<'_>) -> f32 {
        match self.placement {
            ToolbarPlacement::Top => context.theme.layout.top_bar_height,
            ToolbarPlacement::Bottom => context.theme.layout.bottom_bar_height,
        }
    }

    fn content_bounds(
        &self,
        bounds: Rect,
        horizontal: f32,
        vertical: f32,
        context: &mut MeasureContext<'_>,
    ) -> Rect
    where
        Content: View,
    {
        if self.content.owns_toolbar_insets() {
            return bounds;
        }
        let horizontal = horizontal.min(bounds.size.width.max(0.0) / 2.0);
        let vertical = vertical.min(bounds.size.height.max(0.0) / 2.0);
        let slot = Rect::new(
            bounds.origin.x + horizontal,
            bounds.origin.y + vertical,
            (bounds.size.width - horizontal * 2.0).max(0.0),
            (bounds.size.height - vertical * 2.0).max(0.0),
        );
        let measured = self.content.measure(Constraints::loose(slot.size), context);
        centered_content_bounds(slot, measured.height)
    }
}

impl<Content: View> View for Toolbar<Content> {
    fn owns_toolbar_insets(&self) -> bool {
        true
    }

    fn stack_flex_shrink(&self) -> f32 {
        0.0
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        if self.content.owns_toolbar_insets() {
            let size = self.content.measure(constraints, context);
            return constraints.constrain(Size::new(
                size.width,
                size.height.max(self.target_height(context)),
            ));
        }
        let horizontal = context.theme.spacing.extra_large;
        let vertical = match self.placement {
            ToolbarPlacement::Top => context.theme.spacing.small,
            ToolbarPlacement::Bottom => context.theme.spacing.medium,
        };
        let child_constraints = Constraints::loose(Size::new(
            (constraints.maximum.width - horizontal * 2.0).max(0.0),
            (constraints.maximum.height - vertical * 2.0).max(0.0),
        ));
        let content = self.content.measure(child_constraints, context);

        constraints.constrain(Size::new(
            content.width + horizontal * 2.0,
            self.target_height(context)
                .max(content.height + vertical * 2.0),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.record_accessibility(AccessibilityNode::new(AccessibilityRole::Toolbar, bounds));
        Rectangle::new()
            .color(RectangleColor::Surface)
            .paint(bounds, context);
        let horizontal = context.theme.spacing.extra_large;
        let vertical = match self.placement {
            ToolbarPlacement::Top => context.theme.spacing.small,
            ToolbarPlacement::Bottom => context.theme.spacing.medium,
        };
        let content = self.content_bounds(
            bounds,
            horizontal,
            vertical,
            &mut MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            },
        );
        self.content.paint(content, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let horizontal = context.theme.spacing.extra_large;
        let vertical = match self.placement {
            ToolbarPlacement::Top => context.theme.spacing.small,
            ToolbarPlacement::Bottom => context.theme.spacing.medium,
        };
        let content =
            self.content_bounds(bounds, horizontal, vertical, &mut context.measure_context());
        self.content.handle_event(content, event, context)
    }
}

/// Center intrinsic content without narrowing its horizontal layout region.
/// A zero intrinsic height is the legacy contract for custom views that draw
/// directly into their slot; those continue to receive the full slot.
pub(crate) fn centered_content_bounds(slot: Rect, measured_height: f32) -> Rect {
    let height = if measured_height.is_finite() && measured_height > 0.0 {
        measured_height.min(slot.size.height)
    } else {
        slot.size.height
    };
    Rect::new(
        slot.origin.x,
        slot.origin.y + (slot.size.height - height) * 0.5,
        slot.size.width,
        height,
    )
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
    fn toolbar_insets_match_the_chat_screen() {
        let top = Rc::new(Cell::new(None));
        let bottom = Rc::new(Cell::new(None));
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        Toolbar::new(Recorder(top.clone())).paint(Rect::new(0.0, 0.0, 859.0, 48.0), &mut context);
        Toolbar::bottom(Recorder(bottom.clone()))
            .paint(Rect::new(0.0, 0.0, 859.0, 56.0), &mut context);

        assert_eq!(top.get(), Some(Rect::new(24.0, 8.0, 811.0, 32.0)));
        assert_eq!(bottom.get(), Some(Rect::new(24.0, 12.0, 811.0, 32.0)));
    }
    struct Intrinsic {
        height: f32,
        painted: Rc<Cell<Option<Rect>>>,
        dispatched: Rc<Cell<Option<Rect>>>,
    }
    impl View for Intrinsic {
        fn measure(&self, constraints: Constraints, _: &mut MeasureContext<'_>) -> Size {
            constraints.constrain(Size::new(80.0, self.height))
        }
        fn paint(&self, bounds: Rect, _: &mut PaintContext<'_>) {
            self.painted.set(Some(bounds));
        }
        fn handle_event(
            &self,
            bounds: Rect,
            _: &ViewEvent,
            _: &mut EventContext<'_>,
        ) -> EventResult {
            self.dispatched.set(Some(bounds));
            EventResult::Consumed
        }
    }

    #[test]
    fn intrinsic_toolbar_content_is_centered_with_matching_event_geometry() {
        for (placement, bounds) in [
            (ToolbarPlacement::Top, Rect::new(10.0, 20.0, 800.0, 48.0)),
            (ToolbarPlacement::Bottom, Rect::new(10.0, 20.0, 800.0, 56.0)),
            (ToolbarPlacement::Top, Rect::new(10.0, 20.0, 20.0, 10.0)),
        ] {
            let painted = Rc::new(Cell::new(None));
            let dispatched = Rc::new(Cell::new(None));
            let toolbar = Toolbar {
                placement,
                content: Intrinsic {
                    height: 20.0,
                    painted: painted.clone(),
                    dispatched: dispatched.clone(),
                },
            };
            let mut list = DisplayList::new();
            let mut text = TextMeasurer::new();
            toolbar.paint(
                bounds,
                &mut PaintContext::new(&mut list, &Theme::LIGHT, &Typography::DEFAULT, &mut text),
            );
            toolbar.handle_event(
                bounds,
                &ViewEvent::PointerLeft,
                &mut EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut text),
            );
            let actual = painted.get().unwrap();
            assert_eq!(actual, dispatched.get().unwrap());
            assert_eq!(
                actual.origin.y + actual.size.height * 0.5,
                bounds.origin.y + bounds.size.height * 0.5
            );
            assert!(actual.origin.x >= bounds.origin.x);
            assert!(actual.origin.x + actual.size.width <= bounds.origin.x + bounds.size.width);
        }
    }

    #[test]
    fn action_toolbar_keeps_one_inset_when_composed_in_a_toolbar() {
        use super::super::{ActionToolbar, ActionToolbarState, Background, Surface, Text};
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut list = DisplayList::new();
        let bounds = Rect::new(10.0, 20.0, 800.0, 48.0);
        let actions: Box<dyn View> = Box::new(
            Surface::app().content(
                Background::new()
                    .content(ActionToolbar::<u8>::new(ActionToolbarState::new()).title("Title")),
            ),
        );
        let toolbar = Toolbar::new(actions);
        assert_eq!(
            toolbar
                .measure(
                    Constraints::loose(bounds.size),
                    &mut MeasureContext {
                        theme: &theme,
                        typography: &theme.typography,
                        text_measurer: &mut text
                    }
                )
                .height,
            48.0
        );
        toolbar.paint(
            bounds,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
        );
        let frame = list
            .commands()
            .iter()
            .find_map(|command| match command {
                crate::draw_command::DrawCommand::DrawText { command }
                    if command.text == "Title" =>
                {
                    Some(command.bounds)
                }
                _ => None,
            })
            .unwrap();
        let measured = Text::label("Title").measure(
            Constraints::loose(bounds.size),
            &mut MeasureContext {
                theme: &theme,
                typography: &theme.typography,
                text_measurer: &mut text,
            },
        );
        assert_eq!(frame.origin.x, bounds.origin.x + theme.spacing.extra_large);
        assert_eq!(
            frame.origin.y + measured.height * 0.5,
            bounds.origin.y + bounds.size.height * 0.5
        );
    }
}
