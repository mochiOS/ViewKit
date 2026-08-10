//! スクロール可能な表示領域を定義

use std::cell::RefCell;
use std::rc::Rc;

use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::platform::PointerButton;
use crate::theme::ScrollBarTokens;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxis {
    Horizontal,

    #[default]
    Vertical,

    Both,
}

impl ScrollAxis {
    fn allows_horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    fn allows_vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollBarVisibility {
    Hidden,

    #[default]
    Automatic,

    Always,
}

impl ScrollBarVisibility {
    fn should_show(self, overflowing: bool) -> bool {
        match self {
            Self::Hidden => false,
            Self::Automatic => overflowing,
            Self::Always => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollStateInner {
    offset_x: f32,
    offset_y: f32,

    has_cached_layout: bool,
    cached_axis: ScrollAxis,
    cached_viewport_size: Size,
    cached_content_size: Size,

    vertical_drag_offset: Option<f32>,
}

#[derive(Clone, Default)]
pub struct ScrollState {
    inner: Rc<RefCell<ScrollStateInner>>,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn offset(&self) -> Point {
        let inner = self.inner.borrow();

        Point::new(inner.offset_x, inner.offset_y)
    }

    pub fn offset_x(&self) -> f32 {
        self.inner.borrow().offset_x
    }

    pub fn offset_y(&self) -> f32 {
        self.inner.borrow().offset_y
    }

    pub fn set_offset(&self, offset_x: f32, offset_y: f32) {
        let mut inner = self.inner.borrow_mut();

        inner.offset_x = finite_non_negative(offset_x);
        inner.offset_y = finite_non_negative(offset_y);
    }

    pub fn scroll_by(&self, delta_x: f32, delta_y: f32) {
        let mut inner = self.inner.borrow_mut();

        if delta_x.is_finite() {
            inner.offset_x = (inner.offset_x + delta_x).max(0.0);
        }

        if delta_y.is_finite() {
            inner.offset_y = (inner.offset_y + delta_y).max(0.0);
        }
    }

    pub fn reset(&self) {
        self.set_offset(0.0, 0.0);
    }

    fn remember_layout(&self, axis: ScrollAxis, viewport_size: Size, content_size: Size) {
        let mut inner = self.inner.borrow_mut();

        inner.has_cached_layout = true;
        inner.cached_axis = axis;
        inner.cached_viewport_size = viewport_size;
        inner.cached_content_size = content_size;
    }

    fn cached_content_size(&self, axis: ScrollAxis, viewport_size: Size) -> Option<Size> {
        let inner = self.inner.borrow();

        if !inner.has_cached_layout
            || inner.cached_axis != axis
            || inner.cached_viewport_size != viewport_size
        {
            return None;
        }

        Some(inner.cached_content_size)
    }

    fn clamp_offset(&self, axis: ScrollAxis, viewport_size: Size, content_size: Size) -> Point {
        let max_x = (content_size.width - viewport_size.width).max(0.0);

        let max_y = (content_size.height - viewport_size.height).max(0.0);

        let mut inner = self.inner.borrow_mut();

        match axis {
            ScrollAxis::Horizontal => {
                inner.offset_x = inner.offset_x.clamp(0.0, max_x);

                inner.offset_y = 0.0;
            }

            ScrollAxis::Vertical => {
                inner.offset_x = 0.0;

                inner.offset_y = inner.offset_y.clamp(0.0, max_y);
            }

            ScrollAxis::Both => {
                inner.offset_x = inner.offset_x.clamp(0.0, max_x);

                inner.offset_y = inner.offset_y.clamp(0.0, max_y);
            }
        }

        Point::new(inner.offset_x, inner.offset_y)
    }

    fn vertical_drag_offset(&self) -> Option<f32> {
        self.inner.borrow().vertical_drag_offset
    }

    fn begin_vertical_drag(&self, offset: f32) {
        self.inner.borrow_mut().vertical_drag_offset = Some(finite_non_negative(offset));
    }

    fn end_vertical_drag(&self) -> bool {
        self.inner
            .borrow_mut()
            .vertical_drag_offset
            .take()
            .is_some()
    }

    fn set_vertical_offset(&self, offset: f32) {
        self.inner.borrow_mut().offset_y = finite_non_negative(offset);
    }
}

pub struct Scroll {
    state: ScrollState,
    axis: ScrollAxis,
    scrollbar_visibility: ScrollBarVisibility,
    content: Option<StackChild>,
}

impl Scroll {
    pub fn new(state: ScrollState) -> Self {
        Self {
            state,
            axis: ScrollAxis::Vertical,
            scrollbar_visibility: ScrollBarVisibility::Automatic,
            content: None,
        }
    }

    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;

        self
    }

    pub fn scrollbar(mut self, visibility: ScrollBarVisibility) -> Self {
        self.scrollbar_visibility = visibility;

        self
    }

    pub fn content<C>(mut self, content: C) -> Self
    where
        C: IntoStackChild,
    {
        self.content = Some(content.into_stack_child());

        self
    }

    pub fn state(&self) -> &ScrollState {
        &self.state
    }

    #[must_use]
    pub fn vertical<C>(content: C) -> Self
    where
        C: IntoStackChild,
    {
        Self::new(ScrollState::new())
            .axis(ScrollAxis::Vertical)
            .content(content)
    }

    #[must_use]
    pub fn horizontal<C>(content: C) -> Self
    where
        C: IntoStackChild,
    {
        Self::new(ScrollState::new())
            .axis(ScrollAxis::Horizontal)
            .content(content)
    }

    #[must_use]
    pub fn both<C>(content: C) -> Self
    where
        C: IntoStackChild,
    {
        Self::new(ScrollState::new())
            .axis(ScrollAxis::Both)
            .content(content)
    }

    fn content_constraints(&self, viewport_size: Size) -> Constraints {
        let viewport_width = finite_non_negative(viewport_size.width);

        let viewport_height = finite_non_negative(viewport_size.height);

        let minimum_width = if self.axis.allows_horizontal() {
            0.0
        } else {
            viewport_width
        };

        let maximum_width = if self.axis.allows_horizontal() {
            f32::INFINITY
        } else {
            viewport_width
        };

        let minimum_height = if self.axis.allows_vertical() {
            0.0
        } else {
            viewport_height
        };

        let maximum_height = if self.axis.allows_vertical() {
            f32::INFINITY
        } else {
            viewport_height
        };

        Constraints::new(
            Size::new(minimum_width, minimum_height),
            Size::new(maximum_width, maximum_height),
        )
    }

    fn measure_content_size(
        &self,
        content: &StackChild,
        viewport_size: Size,
        context: &mut MeasureContext<'_>,
    ) -> Size {
        let viewport_width = finite_non_negative(viewport_size.width);

        let viewport_height = finite_non_negative(viewport_size.height);

        let measured = content.measure(self.content_constraints(viewport_size), context);

        let measured_width = finite_or(measured.width, viewport_width);

        let measured_height = finite_or(measured.height, viewport_height);

        let width = if self.axis.allows_horizontal() {
            measured_width.max(viewport_width)
        } else {
            viewport_width
        };

        let height = if self.axis.allows_vertical() {
            measured_height.max(viewport_height)
        } else {
            viewport_height
        };

        Size::new(width, height)
    }

    fn content_size_for_event(
        &self,
        content: &StackChild,
        bounds: Rect,
        context: &mut EventContext<'_>,
    ) -> Size {
        if let Some(content_size) = self.state.cached_content_size(self.axis, bounds.size) {
            return content_size;
        }

        let theme = context.theme;
        let typography = context.typography;
        let text_measurer = &mut *context.text_measurer;

        let mut measure_context = MeasureContext {
            theme,
            typography,
            text_measurer,
        };

        let content_size = self.measure_content_size(content, bounds.size, &mut measure_context);

        self.state
            .remember_layout(self.axis, bounds.size, content_size);

        content_size
    }

    fn paint_scrollbars(
        &self,
        bounds: Rect,
        content_size: Size,
        offset: Point,
        context: &mut PaintContext<'_>,
    ) {
        let horizontal_overflow = content_size.width > bounds.size.width;

        let vertical_overflow = content_size.height > bounds.size.height;

        let show_horizontal = self.axis.allows_horizontal()
            && self.scrollbar_visibility.should_show(horizontal_overflow);

        let show_vertical =
            self.axis.allows_vertical() && self.scrollbar_visibility.should_show(vertical_overflow);

        if !show_horizontal && !show_vertical {
            return;
        }

        let tokens = context.theme.scrollbar;

        if show_vertical {
            paint_vertical_scrollbar(
                bounds,
                content_size,
                offset,
                show_horizontal,
                tokens,
                context,
            );
        }

        if show_horizontal {
            paint_horizontal_scrollbar(
                bounds,
                content_size,
                offset,
                show_vertical,
                tokens,
                context,
            );
        }
    }

    fn handle_vertical_scrollbar(
        &self,
        bounds: Rect,
        content_size: Size,
        offset: Point,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !self.axis.allows_vertical() {
            return EventResult::Ignored;
        }

        let horizontal_visible = self.axis.allows_horizontal()
            && self
                .scrollbar_visibility
                .should_show(content_size.width > bounds.size.width);
        let vertical_visible = self
            .scrollbar_visibility
            .should_show(content_size.height > bounds.size.height);
        if !vertical_visible {
            self.state.end_vertical_drag();
            return EventResult::Ignored;
        }

        let Some(geometry) = vertical_scrollbar_geometry(
            bounds,
            content_size,
            offset,
            horizontal_visible,
            context.theme.scrollbar,
        ) else {
            return EventResult::Ignored;
        };

        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if geometry.track.expanded(4.0).contains(*position) => {
                let grab_offset = if geometry.thumb.contains(*position) {
                    position.y - geometry.thumb.origin.y
                } else {
                    geometry.thumb.size.height / 2.0
                };
                self.state.begin_vertical_drag(grab_offset);
                self.update_vertical_drag(position.y, geometry);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerMoved { position } => {
                let Some(grab_offset) = self.state.vertical_drag_offset() else {
                    return EventResult::Ignored;
                };
                self.update_vertical_drag_with_offset(position.y, grab_offset, geometry);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } if self.state.end_vertical_drag() => {
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerLeft if self.state.end_vertical_drag() => EventResult::Consumed,
            _ => EventResult::Ignored,
        }
    }

    fn update_vertical_drag(&self, pointer_y: f32, geometry: ScrollbarGeometry) {
        let grab_offset = self
            .state
            .vertical_drag_offset()
            .unwrap_or(geometry.thumb.size.height / 2.0);
        self.update_vertical_drag_with_offset(pointer_y, grab_offset, geometry);
    }

    fn update_vertical_drag_with_offset(
        &self,
        pointer_y: f32,
        grab_offset: f32,
        geometry: ScrollbarGeometry,
    ) {
        let thumb_travel = (geometry.track.size.height - geometry.thumb.size.height).max(0.0);
        if thumb_travel <= 0.0 || geometry.maximum_offset <= 0.0 {
            self.state.set_vertical_offset(0.0);
            return;
        }
        let thumb_y = (pointer_y - grab_offset - geometry.track.origin.y).clamp(0.0, thumb_travel);
        self.state
            .set_vertical_offset(geometry.maximum_offset * thumb_y / thumb_travel);
    }
}

impl View for Scroll {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        let width = if constraints.maximum.width.is_finite() {
            constraints.maximum.width
        } else {
            constraints.minimum.width
        };

        let height = if constraints.maximum.height.is_finite() {
            constraints.maximum.height
        } else {
            constraints.minimum.height
        };

        constraints.constrain(Size::new(width.max(0.0), height.max(0.0)))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let Some(content) = self.content.as_ref() else {
            return;
        };

        let content_size = {
            let mut measure_context = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };

            self.measure_content_size(content, bounds.size, &mut measure_context)
        };

        self.state
            .remember_layout(self.axis, bounds.size, content_size);

        let offset = self
            .state
            .clamp_offset(self.axis, bounds.size, content_size);

        let content_bounds = Rect::new(
            bounds.origin.x - offset.x,
            bounds.origin.y - offset.y,
            content_size.width,
            content_size.height,
        );

        context
            .display_list
            .push(DrawCommand::PushClip { rect: bounds });

        content.paint(content_bounds, context);

        self.paint_scrollbars(bounds, content_size, offset, context);

        context.display_list.push(DrawCommand::PopClip);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return EventResult::Ignored;
        }

        let Some(content) = self.content.as_ref() else {
            return EventResult::Ignored;
        };

        let content_size = self.content_size_for_event(content, bounds, context);

        let offset = self
            .state
            .clamp_offset(self.axis, bounds.size, content_size);

        let scrollbar_result =
            self.handle_vertical_scrollbar(bounds, content_size, offset, event, context);
        if scrollbar_result.is_consumed() {
            return scrollbar_result;
        }

        let content_bounds = Rect::new(
            bounds.origin.x - offset.x,
            bounds.origin.y - offset.y,
            content_size.width,
            content_size.height,
        );

        if let ViewEvent::PointerMoved { position } = event {
            if !bounds.contains(*position) {
                return content.handle_event(content_bounds, &ViewEvent::PointerLeft, context);
            }
        }

        if !event.requires_broadcast() && !event.is_inside(bounds) {
            return EventResult::Ignored;
        }

        let child_result = content.handle_event(content_bounds, event, context);

        if child_result.is_consumed() {
            return child_result;
        }

        let ViewEvent::Scroll {
            position,
            delta_x,
            delta_y,
        } = event
        else {
            return child_result;
        };

        if !bounds.contains(*position) {
            return EventResult::Ignored;
        }

        let previous_offset = self.state.offset();

        match self.axis {
            ScrollAxis::Horizontal => {
                self.state.scroll_by(*delta_x, 0.0);
            }

            ScrollAxis::Vertical => {
                self.state.scroll_by(0.0, *delta_y);
            }

            ScrollAxis::Both => {
                self.state.scroll_by(*delta_x, *delta_y);
            }
        }

        let current_offset = self
            .state
            .clamp_offset(self.axis, bounds.size, content_size);

        if current_offset == previous_offset {
            return EventResult::Ignored;
        }

        /*
         * スクロールするとviewport内のすべての内容が
         * 移動するため、Scroll全体をdirty領域にします。
         */
        context.request_redraw_in(bounds);

        EventResult::Consumed
    }
}

#[derive(Clone, Copy)]
struct ScrollbarGeometry {
    track: Rect,
    thumb: Rect,
    maximum_offset: f32,
}

fn vertical_scrollbar_geometry(
    bounds: Rect,
    content_size: Size,
    offset: Point,
    horizontal_visible: bool,
    tokens: ScrollBarTokens,
) -> Option<ScrollbarGeometry> {
    let thickness = finite_positive(tokens.thickness);
    let inset = finite_non_negative(tokens.inset);
    if thickness <= 0.0 {
        return None;
    }
    let reserved_bottom = if horizontal_visible {
        thickness + inset
    } else {
        0.0
    };
    let length_inset = finite_non_negative(tokens.length_inset);
    let track_length =
        (bounds.size.height - inset * 2.0 - length_inset * 2.0 - reserved_bottom).max(0.0);
    if track_length <= 0.0 {
        return None;
    }
    let track_x =
        bounds.origin.x + bounds.size.width - inset - thickness - tokens.horizontal_offset;
    let track_y = bounds.origin.y + inset + length_inset;
    let track = Rect::new(track_x, track_y, thickness, track_length);
    let thumb_length = calculate_thumb_length(
        track_length,
        bounds.size.height,
        content_size.height,
        tokens.minimum_thumb_length,
    );
    let maximum_offset = (content_size.height - bounds.size.height).max(0.0);
    let progress = calculate_progress(offset.y, maximum_offset);
    let thumb_travel = (track_length - thumb_length).max(0.0);
    let thumb = Rect::new(
        track_x,
        track_y + thumb_travel * progress,
        thickness,
        thumb_length,
    );
    Some(ScrollbarGeometry {
        track,
        thumb,
        maximum_offset,
    })
}

fn paint_vertical_scrollbar(
    bounds: Rect,
    content_size: Size,
    offset: Point,
    horizontal_visible: bool,
    tokens: ScrollBarTokens,
    context: &mut PaintContext<'_>,
) {
    let Some(geometry) =
        vertical_scrollbar_geometry(bounds, content_size, offset, horizontal_visible, tokens)
    else {
        return;
    };

    context.display_list.push(DrawCommand::FillRoundedRect {
        rect: geometry.track,
        radius: geometry.track.size.width / 2.0,
        color: tokens.track_color,
    });

    context.display_list.push(DrawCommand::FillRoundedRect {
        rect: geometry.thumb,
        radius: geometry.thumb.size.width / 2.0,
        color: tokens.thumb_color,
    });
}

fn paint_horizontal_scrollbar(
    bounds: Rect,
    content_size: Size,
    offset: Point,
    vertical_visible: bool,
    tokens: ScrollBarTokens,
    context: &mut PaintContext<'_>,
) {
    let thickness = finite_positive(tokens.thickness);

    let inset = finite_non_negative(tokens.inset);

    if thickness <= 0.0 {
        return;
    }

    let reserved_right = if vertical_visible {
        thickness + inset
    } else {
        0.0
    };

    let track_length = (bounds.size.width - inset * 2.0 - reserved_right).max(0.0);

    if track_length <= 0.0 {
        return;
    }

    let track_x = bounds.origin.x + inset;

    let track_y = bounds.origin.y + bounds.size.height - inset - thickness;

    let track_rect = Rect::new(track_x, track_y, track_length, thickness);

    context.display_list.push(DrawCommand::FillRoundedRect {
        rect: track_rect,
        radius: thickness / 2.0,
        color: tokens.track_color,
    });

    let thumb_length = calculate_thumb_length(
        track_length,
        bounds.size.width,
        content_size.width,
        tokens.minimum_thumb_length,
    );

    let maximum_offset = (content_size.width - bounds.size.width).max(0.0);

    let progress = calculate_progress(offset.x, maximum_offset);

    let thumb_travel = (track_length - thumb_length).max(0.0);

    let thumb_x = track_x + thumb_travel * progress;

    context.display_list.push(DrawCommand::FillRoundedRect {
        rect: Rect::new(thumb_x, track_y, thumb_length, thickness),
        radius: thickness / 2.0,
        color: tokens.thumb_color,
    });
}

fn calculate_thumb_length(
    track_length: f32,
    viewport_length: f32,
    content_length: f32,
    minimum_thumb_length: f32,
) -> f32 {
    if track_length <= 0.0 {
        return 0.0;
    }

    if content_length <= 0.0 || content_length <= viewport_length {
        return track_length;
    }

    let minimum_thumb_length = finite_non_negative(minimum_thumb_length).min(track_length);

    let natural_length = track_length * viewport_length / content_length;

    natural_length.clamp(minimum_thumb_length, track_length)
}

fn calculate_progress(offset: f32, maximum_offset: f32) -> f32 {
    if maximum_offset <= 0.0 {
        return 0.0;
    }

    (offset / maximum_offset).clamp(0.0, 1.0)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback.max(0.0)
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_positive(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    struct FixedContent;

    impl View for FixedContent {
        fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
            constraints.constrain(Size::new(200.0, 300.0))
        }

        fn paint(&self, _bounds: Rect, _context: &mut PaintContext<'_>) {}
    }

    #[test]
    fn vertical_scroll_preserves_content_taller_than_viewport() {
        let scroll = Scroll::vertical(FixedContent);
        let Some(content) = scroll.content.as_ref() else {
            panic!("scroll content is missing");
        };
        let mut text_measurer = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &Theme::DEFAULT,
            typography: &Typography::DEFAULT,
            text_measurer: &mut text_measurer,
        };

        assert_eq!(
            scroll.measure_content_size(content, Size::new(200.0, 100.0), &mut context),
            Size::new(200.0, 300.0)
        );
    }

    #[test]
    fn vertical_thumb_reaches_both_ends_of_track() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let content = Size::new(200.0, 300.0);
        let tokens = Theme::DEFAULT.scrollbar;
        let Some(start) =
            vertical_scrollbar_geometry(bounds, content, Point::new(0.0, 0.0), false, tokens)
        else {
            panic!("vertical scrollbar geometry is missing");
        };
        let Some(end) =
            vertical_scrollbar_geometry(bounds, content, Point::new(0.0, 200.0), false, tokens)
        else {
            panic!("vertical scrollbar geometry is missing");
        };

        assert_eq!(start.maximum_offset, 200.0);
        assert_eq!(start.thumb.origin.y, start.track.origin.y);
        assert_eq!(
            end.thumb.origin.y + end.thumb.size.height,
            end.track.origin.y + end.track.size.height
        );
    }

    #[test]
    fn vertical_drag_lifecycle_is_explicit() {
        let state = ScrollState::new();
        assert_eq!(state.vertical_drag_offset(), None);
        state.begin_vertical_drag(7.0);
        assert_eq!(state.vertical_drag_offset(), Some(7.0));
        assert!(state.end_vertical_drag());
        assert!(!state.end_vertical_drag());
    }

    #[test]
    fn positive_scroll_delta_moves_content_forward() {
        let state = ScrollState::new();

        state.scroll_by(0.0, 24.0);

        assert_eq!(state.offset_y(), 24.0);
    }
}
