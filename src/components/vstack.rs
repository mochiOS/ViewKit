//! 子Viewを縦方向に配置するVStackを定義

use crate::geometry::Rect;
use crate::layout::{
    paint_stack,
    StackAlignment,
    StackChild,
    StackDirection,
    StackDistribution,
    StackGap,
};
use crate::view::{
    PaintContext,
    View,
};

pub struct VStack {
    children: Vec<StackChild>,
    gap: StackGap,
    alignment: StackAlignment,
    distribution: StackDistribution,
}

impl Default for VStack {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            gap: StackGap::Medium,
            alignment: StackAlignment::Center,
            distribution: StackDistribution::Start,
        }
    }
}

impl VStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(
        mut self,
        child: StackChild,
    ) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(
        mut self,
        children: impl IntoIterator<
            Item = StackChild,
        >,
    ) -> Self {
        self.children.extend(children);
        self
    }

    pub fn gap(
        mut self,
        gap: StackGap,
    ) -> Self {
        self.gap = gap;
        self
    }

    pub fn alignment(
        mut self,
        alignment: StackAlignment,
    ) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn distribution(
        mut self,
        distribution: StackDistribution,
    ) -> Self {
        self.distribution = distribution;
        self
    }
}

impl View for VStack {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        paint_stack(
            StackDirection::Vertical,
            &self.children,
            bounds,
            self.gap,
            self.alignment,
            self.distribution,
            context,
        );
    }
}