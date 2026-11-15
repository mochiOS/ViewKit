//! 子Viewを横方向に配置するHStackを定義

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{
    IntoStackChildren, StackAlignment, StackChild, StackDirection, StackDistribution, StackGap,
    handle_stack_event, layout_stack, measure_stack, paint_stack,
};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

pub struct HStack {
    children: Vec<StackChild>,
    gap: StackGap,
    alignment: StackAlignment,
    distribution: StackDistribution,
}

impl Default for HStack {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            gap: StackGap::Medium,
            alignment: StackAlignment::Center,
            distribution: StackDistribution::Start,
        }
    }
}

impl HStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child<C>(mut self, child: C) -> Self
    where
        C: IntoStackChildren,
    {
        self.children.extend(child.into_stack_children());

        self
    }

    pub fn children<C>(mut self, children: impl IntoIterator<Item = C>) -> Self
    where
        C: IntoStackChildren,
    {
        for child in children {
            self.children.extend(child.into_stack_children());
        }

        self
    }

    pub fn gap(mut self, gap: StackGap) -> Self {
        self.gap = gap;
        self
    }

    pub fn alignment(mut self, alignment: StackAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn distribution(mut self, distribution: StackDistribution) -> Self {
        self.distribution = distribution;
        self
    }

    pub(crate) fn child_bounds_for_event(
        &self,
        bounds: Rect,
        context: &mut EventContext<'_>,
    ) -> Vec<Rect> {
        let theme = context.theme;
        let typography = context.typography;
        let text_measurer = &mut *context.text_measurer;
        let mut measure_context = MeasureContext {
            theme,
            typography,
            text_measurer,
        };
        layout_stack(
            StackDirection::Horizontal,
            &self.children,
            bounds,
            self.gap,
            self.alignment,
            self.distribution,
            theme,
            &mut measure_context,
        )
    }
}

impl View for HStack {
    fn first_baseline(&self, size: Size, context: &mut MeasureContext<'_>) -> Option<f32> {
        crate::layout::stack_first_baseline(
            StackDirection::Horizontal,
            &self.children,
            size,
            self.gap,
            self.alignment,
            self.distribution,
            context,
        )
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let mut size = measure_stack(
            StackDirection::Horizontal,
            &self.children,
            self.gap,
            constraints,
            context,
        );
        if self.alignment == StackAlignment::FirstBaseline {
            let mut ascent = 0.0f32;
            let mut descent = 0.0f32;
            for child in &self.children {
                let child_size = child.measure(Constraints::loose(constraints.maximum), context);
                let baseline = child
                    .first_baseline(child_size, context)
                    .unwrap_or(child_size.height);
                ascent = ascent.max(baseline);
                descent = descent.max(child_size.height - baseline);
            }
            size = constraints.constrain(Size::new(size.width, size.height.max(ascent + descent)));
        }
        size
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        paint_stack(
            StackDirection::Horizontal,
            &self.children,
            bounds,
            self.gap,
            self.alignment,
            self.distribution,
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        handle_stack_event(
            StackDirection::Horizontal,
            &self.children,
            bounds,
            self.gap,
            self.alignment,
            self.distribution,
            event,
            context,
        )
    }
}

#[cfg(test)]
mod baseline_tests {
    use super::*;
    use crate::{
        components::Text,
        theme::Theme,
        typography::{TextMeasurer, Typography},
    };
    #[test]
    fn mixed_language_and_different_font_sizes_share_a_baseline() {
        let theme = Theme::LIGHT;
        let typography = Typography::DEFAULT;
        let mut measurer = TextMeasurer::new();
        let mut cx = MeasureContext {
            theme: &theme,
            typography: &typography,
            text_measurer: &mut measurer,
        };
        let stack = HStack::new()
            .alignment(StackAlignment::FirstBaseline)
            .child(Text::new("English 日本語").font_size(13.).line_height(18.))
            .child(Text::new("Aa 漢字").font_size(24.).line_height(32.));
        let size = stack.measure(Constraints::loose(Size::new(600., 100.)), &mut cx);
        let bounds = layout_stack(
            StackDirection::Horizontal,
            &stack.children,
            Rect::new(0., 0., size.width, size.height),
            stack.gap,
            stack.alignment,
            stack.distribution,
            &theme,
            &mut cx,
        );
        let a = bounds[0].origin.y
            + stack.children[0]
                .first_baseline(bounds[0].size, &mut cx)
                .unwrap();
        let b = bounds[1].origin.y
            + stack.children[1]
                .first_baseline(bounds[1].size, &mut cx)
                .unwrap();
        assert!((a - b).abs() < 0.001);
        assert!(
            bounds
                .iter()
                .all(|r| r.origin.y >= 0. && r.origin.y + r.size.height <= size.height + 0.001)
        );
    }
}
