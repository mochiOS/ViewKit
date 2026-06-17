//! 子Viewを奥行き方向へ重ねるZStackを定義

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
pub enum ZStackAlignment {
    TopLeading,
    Top,
    TopTrailing,
    Leading,

    #[default]
    Center,

    Trailing,
    BottomLeading,
    Bottom,
    BottomTrailing,
}

impl ZStackAlignment {
    fn horizontal_factor(self) -> f32 {
        match self {
            Self::TopLeading
            | Self::Leading
            | Self::BottomLeading => 0.0,

            Self::Top
            | Self::Center
            | Self::Bottom => 0.5,

            Self::TopTrailing
            | Self::Trailing
            | Self::BottomTrailing => 1.0,
        }
    }

    fn vertical_factor(self) -> f32 {
        match self {
            Self::TopLeading
            | Self::Top
            | Self::TopTrailing => 0.0,

            Self::Leading
            | Self::Center
            | Self::Trailing => 0.5,

            Self::BottomLeading
            | Self::Bottom
            | Self::BottomTrailing => 1.0,
        }
    }

    pub(crate) fn child_origin(
        self,
        bounds: Rect,
        child_size: Size,
    ) -> Point {
        let remaining_width =
            bounds.size.width - child_size.width;

        let remaining_height =
            bounds.size.height - child_size.height;

        Point::new(
            bounds.origin.x
                + remaining_width * self.horizontal_factor(),

            bounds.origin.y
                + remaining_height * self.vertical_factor(),
        )
    }

    pub(crate) fn child_bounds(
        self,
        bounds: Rect,
        child_size: Size,
    ) -> Rect {
        let origin =
            self.child_origin(bounds, child_size);

        Rect::new(
            origin.x,
            origin.y,
            child_size.width,
            child_size.height,
        )
    }
}

pub struct ZStack {
    children: Vec<StackChild>,
    alignment: ZStackAlignment,
}

impl Default for ZStack {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            alignment: ZStackAlignment::Center,
        }
    }
}

impl ZStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child<C>(
        mut self,
        child: C,
    ) -> Self
    where
        C: IntoStackChild,
    {
        self.children.push(
            child.into_stack_child(),
        );

        self
    }

    pub fn children<C>(
        mut self,
        children: impl IntoIterator<Item = C>,
    ) -> Self
    where
        C: IntoStackChild,
    {
        self.children.extend(
            children
                .into_iter()
                .map(
                    IntoStackChild::into_stack_child,
                ),
        );

        self
    }

    pub fn alignment(
        mut self,
        alignment: ZStackAlignment,
    ) -> Self {
        self.alignment = alignment;
        self
    }
}

impl View for ZStack {
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

        for child in &self.children {
            let child_size =
                child.overlay_size(bounds.size);

            let child_bounds =
                self.alignment.child_bounds(
                    bounds,
                    child_size,
                );

            child.paint(
                child_bounds,
                context,
            );
        }
    }
}