//! スクロール可能な表示領域

use std::cell::RefCell;
use std::rc::Rc;
use crate::draw_command::DrawCommand;
use crate::geometry::{
    Point,
    Rect,
    Size,
};
use crate::layout::{
    IntoStackChild,
    StackChild,
};
use crate::view::{
    PaintContext,
    View,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxis {
    Horizontal,

    #[default]
    Vertical,

    Both,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollStateInner {
    offset_x: f32,
    offset_y: f32,
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

        Point::new(
            inner.offset_x,
            inner.offset_y,
        )
    }

    pub fn offset_x(&self) -> f32 {
        self.inner.borrow().offset_x
    }

    pub fn offset_y(&self) -> f32 {
        self.inner.borrow().offset_y
    }

    pub fn set_offset(
        &self,
        offset_x: f32,
        offset_y: f32,
    ) {
        let mut inner = self.inner.borrow_mut();

        inner.offset_x =
            finite_non_negative(offset_x);

        inner.offset_y =
            finite_non_negative(offset_y);
    }

    pub fn scroll_by(
        &self,
        delta_x: f32,
        delta_y: f32,
    ) {
        let mut inner = self.inner.borrow_mut();

        if delta_x.is_finite() {
            inner.offset_x =
                (inner.offset_x - delta_x)
                    .max(0.0);
        }

        if delta_y.is_finite() {
            inner.offset_y =
                (inner.offset_y - delta_y)
                    .max(0.0);
        }
    }

    pub fn reset(&self) {
        self.set_offset(
            0.0,
            0.0,
        );
    }

    fn clamp_offset(
        &self,
        axis: ScrollAxis,
        viewport_size: Size,
        content_size: Size,
    ) -> Point {
        let max_x = (
            content_size.width
                - viewport_size.width
        )
            .max(0.0);

        let max_y = (
            content_size.height
                - viewport_size.height
        )
            .max(0.0);

        let mut inner =
            self.inner.borrow_mut();

        match axis {
            ScrollAxis::Horizontal => {
                inner.offset_x =
                    inner.offset_x.clamp(
                        0.0,
                        max_x,
                    );

                inner.offset_y = 0.0;
            }

            ScrollAxis::Vertical => {
                inner.offset_x = 0.0;

                inner.offset_y =
                    inner.offset_y.clamp(
                        0.0,
                        max_y,
                    );
            }

            ScrollAxis::Both => {
                inner.offset_x =
                    inner.offset_x.clamp(
                        0.0,
                        max_x,
                    );

                inner.offset_y =
                    inner.offset_y.clamp(
                        0.0,
                        max_y,
                    );
            }
        }

        Point::new(
            inner.offset_x,
            inner.offset_y,
        )
    }
}

pub struct Scroll {
    state: ScrollState,
    axis: ScrollAxis,
    content: Option<StackChild>,
}

impl Scroll {
    pub fn new(
        state: ScrollState,
    ) -> Self {
        Self {
            state,
            axis: ScrollAxis::Vertical,
            content: None,
        }
    }

    pub fn axis(
        mut self,
        axis: ScrollAxis,
    ) -> Self {
        self.axis = axis;
        self
    }

    pub fn content<C>(
        mut self,
        content: C,
    ) -> Self
    where
        C: IntoStackChild,
    {
        self.content = Some(
            content.into_stack_child(),
        );

        self
    }

    pub fn state(&self) -> &ScrollState {
        &self.state
    }
}

impl View for Scroll {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        if bounds.size.width <= 0.0
            || bounds.size.height <= 0.0
        {
            return;
        }

        let Some(content) =
            self.content.as_ref()
        else {
            return;
        };

        let content_size =
            content.overlay_size(
                bounds.size,
            );

        let offset =
            self.state.clamp_offset(
                self.axis,
                bounds.size,
                content_size,
            );

        let content_bounds = Rect::new(
            bounds.origin.x
                - offset.x,

            bounds.origin.y
                - offset.y,

            content_size.width,
            content_size.height,
        );

        context.display_list.push(
            DrawCommand::PushClip {
                rect: bounds,
            },
        );

        content.paint(
            content_bounds,
            context,
        );

        context.display_list.push(
            DrawCommand::PopClip,
        );
    }
}

fn finite_non_negative(
    value: f32,
) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}