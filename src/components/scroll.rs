//! スクロール可能な表示領域を定義

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
use crate::theme::ScrollBarTokens;
use crate::view::{
    PaintContext,
    View,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
)]
pub enum ScrollAxis {
    Horizontal,

    #[default]
    Vertical,

    Both,
}

impl ScrollAxis {
    fn allows_horizontal(self) -> bool {
        matches!(
            self,
            Self::Horizontal | Self::Both
        )
    }

    fn allows_vertical(self) -> bool {
        matches!(
            self,
            Self::Vertical | Self::Both
        )
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
)]
pub enum ScrollBarVisibility {
    Hidden,

    #[default]
    Automatic,

    Always,
}

impl ScrollBarVisibility {
    fn should_show(
        self,
        overflowing: bool,
    ) -> bool {
        match self {
            Self::Hidden => false,
            Self::Automatic => overflowing,
            Self::Always => true,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
)]
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
        let inner =
            self.inner.borrow();

        Point::new(
            inner.offset_x,
            inner.offset_y,
        )
    }

    pub fn offset_x(&self) -> f32 {
        self.inner
            .borrow()
            .offset_x
    }

    pub fn offset_y(&self) -> f32 {
        self.inner
            .borrow()
            .offset_y
    }

    pub fn set_offset(
        &self,
        offset_x: f32,
        offset_y: f32,
    ) {
        let mut inner =
            self.inner.borrow_mut();

        inner.offset_x =
            finite_non_negative(
                offset_x,
            );

        inner.offset_y =
            finite_non_negative(
                offset_y,
            );
    }

    pub fn scroll_by(
        &self,
        delta_x: f32,
        delta_y: f32,
    ) {
        let mut inner =
            self.inner.borrow_mut();

        if delta_x.is_finite() {
            inner.offset_x =
                (
                    inner.offset_x
                        - delta_x
                )
                    .max(0.0);
        }

        if delta_y.is_finite() {
            inner.offset_y =
                (
                    inner.offset_y
                        - delta_y
                )
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
    scrollbar_visibility:
        ScrollBarVisibility,
    content: Option<StackChild>,
}

impl Scroll {
    pub fn new(
        state: ScrollState,
    ) -> Self {
        Self {
            state,
            axis: ScrollAxis::Vertical,

            scrollbar_visibility:
            ScrollBarVisibility::Automatic,

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

    pub fn scrollbar(
        mut self,
        visibility: ScrollBarVisibility,
    ) -> Self {
        self.scrollbar_visibility =
            visibility;

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

    fn paint_scrollbars(
        &self,
        bounds: Rect,
        content_size: Size,
        offset: Point,
        context: &mut PaintContext<'_>,
    ) {
        let horizontal_overflow =
            content_size.width
                > bounds.size.width;

        let vertical_overflow =
            content_size.height
                > bounds.size.height;

        let show_horizontal =
            self.axis.allows_horizontal()
                && self
                .scrollbar_visibility
                .should_show(
                    horizontal_overflow,
                );

        let show_vertical =
            self.axis.allows_vertical()
                && self
                .scrollbar_visibility
                .should_show(
                    vertical_overflow,
                );

        if !show_horizontal
            && !show_vertical
        {
            return;
        }

        let tokens =
            context.theme.scrollbar;

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

        let content_bounds =
            Rect::new(
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

        self.paint_scrollbars(
            bounds,
            content_size,
            offset,
            context,
        );

        context.display_list.push(
            DrawCommand::PopClip,
        );
    }
}

fn paint_vertical_scrollbar(
    bounds: Rect,
    content_size: Size,
    offset: Point,
    horizontal_visible: bool,
    tokens: ScrollBarTokens,
    context: &mut PaintContext<'_>,
) {
    let thickness =
        finite_positive(
            tokens.thickness,
        );

    let inset =
        finite_non_negative(
            tokens.inset,
        );

    if thickness <= 0.0 {
        return;
    }

    let reserved_bottom =
        if horizontal_visible {
            thickness + inset
        } else {
            0.0
        };

    let track_length = (
        bounds.size.height
            - inset * 2.0
            - reserved_bottom
    )
        .max(0.0);

    if track_length <= 0.0 {
        return;
    }

    let track_x =
        bounds.origin.x
            + bounds.size.width
            - inset
            - thickness;

    let track_y =
        bounds.origin.y
            + inset;

    let track_rect =
        Rect::new(
            track_x,
            track_y,
            thickness,
            track_length,
        );

    context.display_list.push(
        DrawCommand::FillRoundedRect {
            rect: track_rect,
            radius:
            thickness / 2.0,
            color:
            tokens.track_color,
        },
    );

    let thumb_length =
        calculate_thumb_length(
            track_length,
            bounds.size.height,
            content_size.height,
            tokens.minimum_thumb_length,
        );

    let maximum_offset = (
        content_size.height
            - bounds.size.height
    )
        .max(0.0);

    let progress =
        calculate_progress(
            offset.y,
            maximum_offset,
        );

    let thumb_travel =
        (
            track_length
                - thumb_length
        )
            .max(0.0);

    let thumb_y =
        track_y
            + thumb_travel
            * progress;

    context.display_list.push(
        DrawCommand::FillRoundedRect {
            rect: Rect::new(
                track_x,
                thumb_y,
                thickness,
                thumb_length,
            ),

            radius:
            thickness / 2.0,

            color:
            tokens.thumb_color,
        },
    );
}

fn paint_horizontal_scrollbar(
    bounds: Rect,
    content_size: Size,
    offset: Point,
    vertical_visible: bool,
    tokens: ScrollBarTokens,
    context: &mut PaintContext<'_>,
) {
    let thickness =
        finite_positive(
            tokens.thickness,
        );

    let inset =
        finite_non_negative(
            tokens.inset,
        );

    if thickness <= 0.0 {
        return;
    }

    let reserved_right =
        if vertical_visible {
            thickness + inset
        } else {
            0.0
        };

    let track_length = (
        bounds.size.width
            - inset * 2.0
            - reserved_right
    )
        .max(0.0);

    if track_length <= 0.0 {
        return;
    }

    let track_x =
        bounds.origin.x
            + inset;

    let track_y =
        bounds.origin.y
            + bounds.size.height
            - inset
            - thickness;

    let track_rect =
        Rect::new(
            track_x,
            track_y,
            track_length,
            thickness,
        );

    context.display_list.push(
        DrawCommand::FillRoundedRect {
            rect: track_rect,
            radius:
            thickness / 2.0,
            color:
            tokens.track_color,
        },
    );

    let thumb_length =
        calculate_thumb_length(
            track_length,
            bounds.size.width,
            content_size.width,
            tokens.minimum_thumb_length,
        );

    let maximum_offset = (
        content_size.width
            - bounds.size.width
    )
        .max(0.0);

    let progress =
        calculate_progress(
            offset.x,
            maximum_offset,
        );

    let thumb_travel =
        (
            track_length
                - thumb_length
        )
            .max(0.0);

    let thumb_x =
        track_x
            + thumb_travel
            * progress;

    context.display_list.push(
        DrawCommand::FillRoundedRect {
            rect: Rect::new(
                thumb_x,
                track_y,
                thumb_length,
                thickness,
            ),

            radius:
            thickness / 2.0,

            color:
            tokens.thumb_color,
        },
    );
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

    if content_length <= 0.0
        || content_length
        <= viewport_length
    {
        return track_length;
    }

    let minimum_thumb_length =
        finite_non_negative(
            minimum_thumb_length,
        )
            .min(
                track_length,
            );

    let natural_length =
        track_length
            * viewport_length
            / content_length;

    natural_length.clamp(
        minimum_thumb_length,
        track_length,
    )
}

fn calculate_progress(
    offset: f32,
    maximum_offset: f32,
) -> f32 {
    if maximum_offset <= 0.0 {
        return 0.0;
    }

    (
        offset
            / maximum_offset
    )
        .clamp(
            0.0,
            1.0,
        )
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

fn finite_positive(
    value: f32,
) -> f32 {
    if value.is_finite()
        && value > 0.0
    {
        value
    } else {
        0.0
    }
}