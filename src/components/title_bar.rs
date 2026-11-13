//! Ordinary ViewKit components arranged beside platform-owned window controls.
use crate::{
    components::{Rectangle, RectangleColor},
    decoration::{DecorationConfig, DecorationError, DecorationLayout, DecorationMode},
    draw_command::DrawCommand,
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    view::{Constraints, MeasureContext, PaintContext, View},
};
use std::rc::Rc;

/// Application title bar content. Construct it from the same State-backed
/// components as the body in `App::window_for`; rebuilding the view tree does
/// not recreate the platform window. Keep component State in the application
/// just as for body components, including dynamically reordered tabs.
#[derive(Clone)]
pub struct TitleBar {
    height: f32,
    background: RectangleColor,
    separator: bool,
    slots: [Option<Rc<dyn View>>; 3],
}
impl core::fmt::Debug for TitleBar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TitleBar")
            .field("height", &self.height)
            .field("background", &self.background)
            .field("separator", &self.separator)
            .finish_non_exhaustive()
    }
}
impl Default for TitleBar {
    fn default() -> Self {
        Self::new()
    }
}
impl TitleBar {
    /// Creates a title bar using the system's minimum height and background.
    pub fn new() -> Self {
        Self {
            height: 40.0,
            background: RectangleColor::SubtleSurface,
            separator: true,
            slots: [None, None, None],
        }
    }
    /// Requests a logical height. Invalid values are rejected at window creation.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
    /// Uses an existing theme color or a custom color for the whole title bar.
    pub fn background(mut self, color: RectangleColor) -> Self {
        self.background = color;
        self
    }
    /// Shows or hides the standard separator below the title bar.
    pub fn separator(mut self, visible: bool) -> Self {
        self.separator = visible;
        self
    }
    /// Places a normal view at the leading edge.
    pub fn leading(mut self, view: impl View + 'static) -> Self {
        self.slots[0] = Some(Rc::new(view));
        self
    }
    /// Places a normal view in the center, constrained to avoid adjacent views.
    pub fn center(mut self, view: impl View + 'static) -> Self {
        self.slots[1] = Some(Rc::new(view));
        self
    }
    /// Places a normal view before the reserved system control area.
    pub fn trailing(mut self, view: impl View + 'static) -> Self {
        self.slots[2] = Some(Rc::new(view));
        self
    }
    pub(crate) fn validate(&self) -> Result<(), DecorationError> {
        if !self.height.is_finite() || !(1.0..=4096.0).contains(&self.height) {
            return Err(DecorationError::InvalidHeight);
        }
        Ok(())
    }
    pub(crate) fn config(&self, theme: &crate::theme::Theme) -> DecorationConfig {
        let background = match self.background {
            RectangleColor::Background => theme.colors.background,
            RectangleColor::Surface => theme.colors.surface,
            RectangleColor::SubtleSurface => theme.colors.surface_subtle,
            RectangleColor::ElevatedSurface => theme.colors.elevated_surface,
            RectangleColor::Accent => theme.colors.accent,
            RectangleColor::Destructive => theme.colors.destructive,
            RectangleColor::Custom(c) => c,
        };
        DecorationConfig {
            mode: DecorationMode::Integrated,
            height: self.height,
            background,
            separator: self.separator,
            resizable: true,
        }
    }
    fn slots(&self, b: Rect, cx: &mut MeasureContext<'_>) -> [Rect; 3] {
        let widths = self.slots.each_ref().map(|v| {
            v.as_ref().map_or(0.0, |v| {
                v.measure(Constraints::loose(b.size), cx)
                    .width
                    .max(0.0)
                    .min(b.size.width)
            })
        });
        let total = widths.iter().sum::<f32>();
        let shrink = if total > b.size.width && total > 0.0 {
            b.size.width / total
        } else {
            1.0
        };
        let [l, c, r] = widths.map(|w| w * shrink);
        let center = ((b.size.width - c) * 0.5)
            .max(l)
            .min((b.size.width - r - c).max(l));
        let widths = [l, c, r];
        let x = [
            b.origin.x,
            b.origin.x + center,
            b.origin.x + b.size.width - r,
        ];
        core::array::from_fn(|index| {
            let height = self.slots[index].as_ref().map_or(0.0, |view| {
                view.measure(
                    Constraints::loose(Size::new(widths[index], b.size.height)),
                    cx,
                )
                .height
                .max(0.0)
                .min(b.size.height)
            });
            Rect::new(
                x[index],
                b.origin.y + (b.size.height - height) * 0.5,
                widths[index],
                height,
            )
        })
    }
    pub(crate) fn paint_in(&self, b: Rect, layout: DecorationLayout, cx: &mut PaintContext<'_>) {
        let custom = layout.custom_bounds(b);
        cx.display_list.push(DrawCommand::PushClip { rect: b });
        Rectangle::new().color(self.background).paint(custom, cx);
        let slots = self.slots(
            custom,
            &mut MeasureContext {
                theme: cx.theme,
                typography: cx.typography,
                text_measurer: cx.text_measurer,
            },
        );
        for (view, bounds) in self.slots.iter().zip(slots) {
            if let Some(view) = view {
                // Standard component overlays (tooltips, IME candidates,
                // popovers) may extend into the content area. The separate
                // compositor-owned surface protects the system control strip.
                view.paint(bounds, cx);
            }
        }
        if self.separator {
            Rectangle::new()
                .color(RectangleColor::Custom(cx.theme.colors.border))
                .paint(
                    Rect::new(
                        custom.origin.x,
                        custom.origin.y + custom.size.height - 1.0,
                        custom.size.width,
                        1.0,
                    ),
                    cx,
                );
        }
        cx.display_list.push(DrawCommand::PopClip);
    }
    pub(crate) fn event_in(
        &self,
        b: Rect,
        layout: DecorationLayout,
        e: &ViewEvent,
        cx: &mut EventContext<'_>,
    ) -> EventResult {
        let custom = layout.custom_bounds(b);
        if e.position().is_some_and(|p| {
            p.y >= b.origin.y && p.y < b.origin.y + custom.size.height && !custom.contains(p)
        }) && !e.requires_broadcast()
        {
            return EventResult::Ignored;
        }
        let slots = self.slots(
            custom,
            &mut MeasureContext {
                theme: cx.theme,
                typography: cx.typography,
                text_measurer: cx.text_measurer,
            },
        );
        let mut result = EventResult::Ignored;
        for (view, bounds) in self.slots.iter().zip(slots).rev() {
            if let Some(view) = view {
                let next = view.handle_event(bounds, e, cx);
                result = result.merge(next);
                if next.is_consumed() && !e.requires_broadcast() {
                    break;
                }
            }
        }
        result
    }
}

pub(crate) struct DecoratedRoot<V> {
    pub body: V,
    pub titlebar: Option<TitleBar>,
    pub layout: DecorationLayout,
}
impl<V: View> DecoratedRoot<V> {
    fn content_bounds(&self, b: Rect) -> Rect {
        let h = if self.titlebar.is_some() {
            self.layout.height.min(b.size.height)
        } else {
            0.0
        };
        Rect::new(
            b.origin.x,
            b.origin.y + h,
            b.size.width,
            (b.size.height - h).max(0.0),
        )
    }
}
impl<V: View> View for DecoratedRoot<V> {
    fn paint(&self, b: Rect, cx: &mut PaintContext<'_>) {
        let content = self.content_bounds(b);
        cx.display_list
            .push(DrawCommand::PushClip { rect: content });
        self.body.paint(content, cx);
        cx.display_list.push(DrawCommand::PopClip);
        if let Some(bar) = &self.titlebar
            && self.layout.height > 0.0
        {
            bar.paint_in(b, self.layout, cx);
        }
    }
    fn handle_event(&self, b: Rect, e: &ViewEvent, cx: &mut EventContext<'_>) -> EventResult {
        let mut result = EventResult::Ignored;
        if let Some(bar) = &self.titlebar
            && self.layout.height > 0.0
        {
            result = bar.event_in(b, self.layout, e, cx);
        }
        if result.is_consumed() && !e.requires_broadcast() {
            return result;
        }
        let content = self.content_bounds(b);
        if e.position().is_none_or(|p| content.contains(p)) || e.requires_broadcast() {
            result = result.merge(self.body.handle_event(content, e, cx));
        }
        result
    }
}

impl PartialEq for TitleBar {
    fn eq(&self, other: &Self) -> bool {
        self.height == other.height
            && self.background == other.background
            && self.separator == other.separator
            && self
                .slots
                .iter()
                .zip(&other.slots)
                .all(|(a, b)| match (a, b) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    _ => false,
                })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::WindowOptions,
        components::{DragRegion, Identified, TextField, TextFieldInteractionState},
        decoration::{WindowInteraction, WindowState},
        geometry::Point,
        platform::PointerButton,
        runtime::ComponentInstanceId,
        theme::Theme,
        typography::TextMeasurer,
    };
    use std::cell::Cell;

    struct Probe {
        consume: bool,
        paints: Rc<Cell<usize>>,
    }
    impl View for Probe {
        fn measure(&self, c: Constraints, _: &mut MeasureContext<'_>) -> Size {
            c.constrain(Size::new(300.0, 24.0))
        }
        fn paint(&self, _: Rect, _: &mut PaintContext<'_>) {
            self.paints.set(self.paints.get() + 1);
        }
        fn handle_event(&self, _: Rect, e: &ViewEvent, _: &mut EventContext<'_>) -> EventResult {
            if self.consume && matches!(e, ViewEvent::PointerPressed { .. }) {
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        }
    }
    fn probe(consume: bool) -> Probe {
        Probe {
            consume,
            paints: Rc::new(Cell::new(0)),
        }
    }
    fn layout() -> DecorationLayout {
        DecorationLayout {
            height: 44.0,
            controls_width: 112.0,
            state: WindowState::Normal,
        }
    }

    #[test]
    fn decoration_selection_validates_both_builder_orders_and_bad_heights() {
        assert_eq!(
            WindowOptions::default().decoration(),
            DecorationMode::System
        );
        assert_eq!(
            WindowOptions::default()
                .titlebar(TitleBar::new())
                .decoration(),
            DecorationMode::Integrated
        );
        for options in [
            WindowOptions::default()
                .decoration_mode(DecorationMode::System)
                .titlebar(TitleBar::new()),
            WindowOptions::default()
                .titlebar(TitleBar::new())
                .decoration_mode(DecorationMode::System),
        ] {
            assert_eq!(options.validate(), Err(DecorationError::ConflictingMode));
        }
        for h in [f32::NAN, f32::INFINITY, -1.0, 0.0, 4097.0] {
            assert_eq!(
                TitleBar::new().height(h).validate(),
                Err(DecorationError::InvalidHeight)
            );
        }
        assert!(
            WindowOptions::default()
                .titlebar(TitleBar::new())
                .fullscreen(true)
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn lanes_shrink_without_overlapping_or_using_system_controls() {
        let bar = TitleBar::new()
            .leading(probe(false))
            .center(probe(false))
            .trailing(probe(false));
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        for width in [0.0, 50.0, 112.0, 140.0, 640.0, 1920.0] {
            let custom = layout().custom_bounds(Rect::new(0.0, 0.0, width, 800.0));
            let slots = bar.slots(
                custom,
                &mut MeasureContext {
                    theme: &theme,
                    typography: &theme.typography,
                    text_measurer: &mut text,
                },
            );
            assert!((slots[0].origin.x + slots[0].size.width) <= slots[1].origin.x + 0.001);
            assert!((slots[1].origin.x + slots[1].size.width) <= slots[2].origin.x + 0.001);
            assert!(
                (slots[2].origin.x + slots[2].size.width)
                    <= (custom.origin.x + custom.size.width) + 0.001
            );
        }
    }

    #[test]
    fn controls_precede_drag_fallback_and_tokens_are_single_use() {
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let press = ViewEvent::PointerPressed {
            position: Point::new(10.0, 10.0),
            button: PointerButton::Primary,
        };
        let b = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        cx.input_serial = 73;
        DragRegion::new(probe(true)).handle_event(b, &press, &mut cx);
        assert!(cx.take_window_interaction().is_none());
        DragRegion::new(probe(false)).handle_event(b, &press, &mut cx);
        assert_eq!(
            cx.take_window_interaction(),
            Some((73, WindowInteraction::Move))
        );
        assert!(!cx.request_window_move());
        let mut unauthenticated = EventContext::new(&theme, &theme.typography, &mut text);
        assert!(!unauthenticated.request_window_move());
    }

    #[test]
    fn reserved_area_does_not_dispatch_to_application_components() {
        let bar = TitleBar::new().trailing(DragRegion::new(probe(false)));
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        cx.input_serial = 74;
        let b = Rect::new(0.0, 0.0, 640.0, 400.0);
        let event = ViewEvent::PointerPressed {
            position: Point::new(630.0, 10.0),
            button: PointerButton::Primary,
        };
        assert_eq!(
            bar.event_in(b, layout(), &event, &mut cx),
            EventResult::Ignored
        );
        assert!(cx.take_window_interaction().is_none());
    }

    #[test]
    fn removing_an_identified_field_releases_its_focus_and_ime_state() {
        let state = TextFieldInteractionState::new();
        state.set_focused(true);
        let field = Identified::new(
            ComponentInstanceId(10),
            TextField::with_interaction(state.clone()),
        );
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        let b = Rect::new(0.0, 0.0, 300.0, 40.0);
        drop(cx);
        let mut nodes = Vec::new();
        let mut list = crate::draw_command::DisplayList::new();
        field.paint(
            b,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text)
                .with_accessibility_nodes(&mut nodes),
        );
        let retained = nodes.iter().filter_map(|node| node.identity).collect();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        field.handle_event(b, &ViewEvent::ReconcileComponents { retained }, &mut cx);
        assert!(state.is_focused());
        field.handle_event(
            b,
            &ViewEvent::ReconcileComponents {
                retained: vec![crate::accessibility::AccessibilityIdentity::Component {
                    id: 20,
                    element: 0,
                }],
            },
            &mut cx,
        );
        assert!(!state.is_focused());
    }
}

#[cfg(test)]
mod overlay_tests {
    use super::*;
    use crate::{geometry::Point, platform::PointerButton, theme::Theme, typography::TextMeasurer};
    struct ExpandingComponent;
    impl View for ExpandingComponent {
        fn measure(&self, c: Constraints, _: &mut MeasureContext<'_>) -> Size {
            c.constrain(Size::new(100.0, 40.0))
        }
        fn paint(&self, b: Rect, cx: &mut PaintContext<'_>) {
            Rectangle::new()
                .color(RectangleColor::Accent)
                .paint(Rect::new(b.origin.x, b.origin.y + 44.0, 100.0, 60.0), cx);
        }
        fn handle_event(&self, b: Rect, e: &ViewEvent, _: &mut EventContext<'_>) -> EventResult {
            if matches!(e,ViewEvent::PointerPressed {position,button:PointerButton::Primary} if Rect::new(b.origin.x,b.origin.y+44.0,100.0,60.0).contains(*position))
            {
                EventResult::Consumed
            } else {
                EventResult::Ignored
            }
        }
    }
    #[test]
    fn titlebar_component_overlays_can_paint_and_receive_input_over_body_content() {
        let layout = DecorationLayout {
            height: 44.0,
            controls_width: 112.0,
            ..Default::default()
        };
        let bar = TitleBar::new().leading(ExpandingComponent);
        let b = Rect::new(0.0, 0.0, 640.0, 480.0);
        let root = DecoratedRoot {
            body: Rectangle::new().color(RectangleColor::Surface),
            titlebar: Some(bar),
            layout,
        };
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut list = crate::draw_command::DisplayList::new();
        root.paint(
            b,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
        );
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        assert_eq!(
            root.handle_event(
                b,
                &ViewEvent::PointerPressed {
                    position: Point::new(10.0, 60.0),
                    button: PointerButton::Primary
                },
                &mut cx
            ),
            EventResult::Consumed
        );
        assert!(
            !list
                .commands()
                .iter()
                .any(|cmd| matches!(cmd,DrawCommand::PushClip {rect} if rect.size.height==44.0))
        );
    }
}

#[cfg(test)]
mod intrinsic_height_tests {
    use super::*;
    use crate::{components::Button, theme::Theme, typography::TextMeasurer};
    #[test]
    fn standard_buttons_keep_their_intrinsic_height_and_are_centered() {
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut cx = MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut text,
        };
        let button = Button::new("Menu");
        let measured = button.measure(Constraints::loose(Size::new(500.0, 44.0)), &mut cx);
        let bar = TitleBar::new().leading(button);
        let bounds = Rect::new(0.0, 0.0, 500.0, 44.0);
        let slots = bar.slots(bounds, &mut cx);
        assert_eq!(slots[0].size.height, measured.height);
        assert_eq!(slots[0].origin.y + slots[0].size.height * 0.5, 22.0);
    }
}
