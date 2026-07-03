//! ViewKitのレイアウト型とui_layout-rsとの接続を定義

use ui_layout::{
    AlignItems, Display, FlexDirection, ItemStyle, JustifyContent, LayoutNode, Length, SizeStyle,
    Style,
};

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::theme::{DividerThickness, DividerTokens, SpacingTokens, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutLength {
    #[default]
    Auto,

    Fixed(f32),
}

impl LayoutLength {
    #[allow(unused)]
    fn to_ui_length(self) -> Length {
        match self {
            Self::Auto => Length::Auto,

            Self::Fixed(value) => {
                if value.is_finite() {
                    Length::Px(value.max(0.0))
                } else {
                    Length::Px(0.0)
                }
            }
        }
    }

    fn resolve_overlay(self, available: f32) -> f32 {
        match self {
            Self::Auto => {
                if available.is_finite() {
                    available.max(0.0)
                } else {
                    0.0
                }
            }

            Self::Fixed(value) => {
                if value.is_finite() {
                    value.max(0.0)
                } else {
                    0.0
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackAlignment {
    Start,

    #[default]
    Center,

    End,

    Stretch,
}

impl StackAlignment {
    fn to_ui_alignment(self) -> AlignItems {
        match self {
            Self::Start => AlignItems::Start,
            Self::Center => AlignItems::Center,
            Self::End => AlignItems::End,
            Self::Stretch => AlignItems::Stretch,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackDistribution {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl StackDistribution {
    fn to_ui_justification(self) -> JustifyContent {
        match self {
            Self::Start => JustifyContent::Start,

            Self::Center => JustifyContent::Center,

            Self::End => JustifyContent::End,

            Self::SpaceBetween => JustifyContent::SpaceBetween,

            Self::SpaceAround => JustifyContent::SpaceAround,

            Self::SpaceEvenly => JustifyContent::SpaceEvenly,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum StackGap {
    None,

    ExtraSmall,

    Small,

    #[default]
    Medium,

    Large,

    ExtraLarge,

    DoubleExtraLarge,

    Custom(f32),
}

impl StackGap {
    pub fn resolve(self, tokens: &SpacingTokens) -> f32 {
        match self {
            Self::None => 0.0,

            Self::ExtraSmall => tokens.extra_small,

            Self::Small => tokens.small,

            Self::Medium => tokens.medium,

            Self::Large => tokens.large,

            Self::ExtraLarge => tokens.extra_large,

            Self::DoubleExtraLarge => tokens.double_extra_large,

            Self::Custom(value) => value.max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum StackChildKind {
    Normal,

    Divider { thickness: DividerThickness },
}

pub struct StackChild {
    view: Box<dyn View>,

    width: LayoutLength,
    height: LayoutLength,

    flex_grow: f32,
    flex_shrink: f32,

    kind: StackChildKind,
}

impl StackChild {
    pub fn new<V>(view: V) -> Self
    where
        V: View + 'static,
    {
        Self {
            view: Box::new(view),

            width: LayoutLength::Auto,
            height: LayoutLength::Auto,

            flex_grow: 0.0,
            flex_shrink: 1.0,

            kind: StackChildKind::Normal,
        }
    }

    pub(crate) fn spacer() -> Self {
        Self {
            view: Box::new(EmptyView),

            width: LayoutLength::Auto,
            height: LayoutLength::Auto,

            flex_grow: 1.0,
            flex_shrink: 1.0,

            kind: StackChildKind::Normal,
        }
    }

    pub(crate) fn measure(
        &self,
        constraints: Constraints,
        context: &mut MeasureContext<'_>,
    ) -> Size {
        let (minimum_width, maximum_width) = resolve_axis_constraints(
            self.width,
            constraints.minimum.width,
            constraints.maximum.width,
        );

        let (minimum_height, maximum_height) = resolve_axis_constraints(
            self.height,
            constraints.minimum.height,
            constraints.maximum.height,
        );

        let child_constraints = Constraints::new(
            Size::new(minimum_width, minimum_height),
            Size::new(maximum_width, maximum_height),
        );

        let measured = self.view.measure(child_constraints, context);

        constraints.constrain(Size::new(
            match self.width {
                LayoutLength::Auto => measured.width,

                LayoutLength::Fixed(value) => sanitize_length(value),
            },
            match self.height {
                LayoutLength::Auto => measured.height,

                LayoutLength::Fixed(value) => sanitize_length(value),
            },
        ))
    }

    pub(crate) fn divider<V>(view: V, thickness: DividerThickness) -> Self
    where
        V: View + 'static,
    {
        Self {
            view: Box::new(view),

            width: LayoutLength::Auto,
            height: LayoutLength::Auto,

            flex_grow: 0.0,
            flex_shrink: 0.0,

            kind: StackChildKind::Divider { thickness },
        }
    }

    pub(crate) fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.view.handle_event(bounds, event, context)
    }

    #[allow(unused)]
    fn create_layout_node(
        &self,
        direction: StackDirection,
        available_size: Size,
        theme: &Theme,
    ) -> LayoutNode {
        let mut width = self.width.to_ui_length();

        let mut height = self.height.to_ui_length();

        if matches!(self.kind, StackChildKind::Divider { .. }) {
            match direction {
                StackDirection::Vertical => {
                    width = Length::Px(available_size.width.max(0.0));

                    height = Length::Px(theme.divider.thickness.max(0.0));
                }

                StackDirection::Horizontal => {
                    width = Length::Px(theme.divider.thickness.max(0.0));

                    height = Length::Px(available_size.height.max(0.0));
                }
            }
        }

        let flex_basis = match direction {
            StackDirection::Horizontal => width.clone(),

            StackDirection::Vertical => height.clone(),
        };

        LayoutNode::new(Style {
            display: Display::Block,

            size: SizeStyle {
                width,
                height,
                ..SizeStyle::default()
            },

            item_style: ItemStyle {
                flex_grow: self.flex_grow.max(0.0),

                flex_shrink: self.flex_shrink.max(0.0),

                flex_basis,

                align_self: None,
            },

            ..Style::default()
        })
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = LayoutLength::Fixed(width);

        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = LayoutLength::Fixed(height);

        self
    }

    pub fn frame(mut self, width: f32, height: f32) -> Self {
        self.width = LayoutLength::Fixed(width);

        self.height = LayoutLength::Fixed(height);

        self
    }

    pub fn flex_grow(mut self, value: f32) -> Self {
        self.flex_grow = value.max(0.0);

        self
    }

    pub fn flex_shrink(mut self, value: f32) -> Self {
        self.flex_shrink = value.max(0.0);

        self
    }

    pub(crate) fn overlay_size(&self, available: Size) -> Size {
        Size::new(
            self.width.resolve_overlay(available.width),
            self.height.resolve_overlay(available.height),
        )
    }

    pub(crate) fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.view.paint(bounds, context);
    }

    fn layout_node(
        &self,
        direction: StackDirection,
        bounds: Rect,
        measured_size: Size,
        divider_tokens: &DividerTokens,
    ) -> LayoutNode {
        let mut width = measured_ui_length(self.width, measured_size.width);

        let mut height = measured_ui_length(self.height, measured_size.height);

        if let StackChildKind::Divider { thickness } = self.kind {
            let thickness = thickness.resolve(divider_tokens).max(0.0);

            match direction {
                StackDirection::Vertical => {
                    width = Length::Px(bounds.size.width.max(0.0));
                    height = Length::Px(thickness);
                }

                StackDirection::Horizontal => {
                    width = Length::Px(thickness);
                    height = Length::Px(bounds.size.height.max(0.0));
                }
            }
        }

        let flex_basis = match direction {
            StackDirection::Horizontal => width.clone(),
            StackDirection::Vertical => height.clone(),
        };

        LayoutNode::new(Style {
            display: Display::Block,

            size: SizeStyle {
                width,
                height,

                ..Default::default()
            },

            item_style: ItemStyle {
                flex_grow: self.flex_grow.max(0.0),
                flex_shrink: self.flex_shrink.max(0.0),
                flex_basis,
                align_self: None,
            },

            ..Default::default()
        })
    }
}

pub trait IntoStackChild {
    fn into_stack_child(self) -> StackChild;
}

impl IntoStackChild for StackChild {
    fn into_stack_child(self) -> StackChild {
        self
    }
}

impl<V> IntoStackChild for V
where
    V: View + 'static,
{
    fn into_stack_child(self) -> StackChild {
        StackChild::new(self)
    }
}

pub trait IntoStackChildren {
    fn into_stack_children(self) -> Vec<StackChild>;
}

impl<T> IntoStackChildren for T
where
    T: IntoStackChild,
{
    fn into_stack_children(self) -> Vec<StackChild> {
        vec![self.into_stack_child()]
    }
}

pub trait ViewExt: View + Sized + 'static {
    fn layout(self) -> StackChild {
        StackChild::new(self)
    }

    fn frame(self, width: f32, height: f32) -> StackChild {
        StackChild::new(self).frame(width, height)
    }

    fn width(self, width: f32) -> StackChild {
        StackChild::new(self).width(width)
    }

    fn height(self, height: f32) -> StackChild {
        StackChild::new(self).height(height)
    }
}

impl<T> ViewExt for T where T: View + Sized + 'static {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StackDirection {
    Vertical,
    Horizontal,
}

impl StackDirection {
    fn to_ui_flex_direction(self) -> FlexDirection {
        match self {
            Self::Horizontal => FlexDirection::Row,
            Self::Vertical => FlexDirection::Column,
        }
    }
}

impl StackAlignment {
    #[allow(unused)]
    fn to_ui_align_items(self) -> AlignItems {
        match self {
            Self::Start => AlignItems::Start,
            Self::Center => AlignItems::Center,
            Self::End => AlignItems::End,
            Self::Stretch => AlignItems::Stretch,
        }
    }
}

impl StackDistribution {
    #[allow(unused)]
    fn to_ui_justify_content(self) -> JustifyContent {
        match self {
            Self::Start => JustifyContent::Start,
            Self::Center => JustifyContent::Center,
            Self::End => JustifyContent::End,

            Self::SpaceBetween => JustifyContent::SpaceBetween,

            Self::SpaceAround => JustifyContent::SpaceAround,

            Self::SpaceEvenly => JustifyContent::SpaceEvenly,
        }
    }
}

pub(crate) fn measure_stack(
    direction: StackDirection,
    children: &[StackChild],
    gap: StackGap,
    constraints: Constraints,
    context: &mut MeasureContext<'_>,
) -> Size {
    if children.is_empty() {
        return constraints.constrain(Size::ZERO);
    }

    let resolved_gap = gap.resolve(&context.theme.spacing).max(0.0);
    let divider_tokens = context.theme.divider;
    let child_constraints = Constraints::loose(constraints.maximum);

    let mut main_size = 0.0_f32;
    let mut cross_size = 0.0_f32;

    for child in children {
        let measured = match child.kind {
            StackChildKind::Normal => child.measure(child_constraints, context),

            StackChildKind::Divider { thickness } => {
                let thickness = thickness.resolve(&divider_tokens).max(0.0);

                match direction {
                    StackDirection::Horizontal => Size::new(thickness, 0.0),

                    StackDirection::Vertical => Size::new(0.0, thickness),
                }
            }
        };

        match direction {
            StackDirection::Horizontal => {
                main_size += measured.width;

                cross_size = cross_size.max(measured.height);
            }

            StackDirection::Vertical => {
                main_size += measured.height;

                cross_size = cross_size.max(measured.width);
            }
        }
    }

    main_size += resolved_gap * children.len().saturating_sub(1) as f32;

    let measured = match direction {
        StackDirection::Horizontal => Size::new(main_size, cross_size),

        StackDirection::Vertical => Size::new(cross_size, main_size),
    };

    constraints.constrain(measured)
}

pub(crate) fn paint_stack(
    direction: StackDirection,
    children: &[StackChild],
    bounds: Rect,
    gap: StackGap,
    alignment: StackAlignment,
    distribution: StackDistribution,
    context: &mut PaintContext<'_>,
) {
    let child_bounds = {
        let mut measure_context = MeasureContext {
            theme: context.theme,

            typography: context.typography,

            text_measurer: &mut *context.text_measurer,
        };

        layout_stack(
            direction,
            children,
            bounds,
            gap,
            alignment,
            distribution,
            context.theme,
            &mut measure_context,
        )
    };

    for (child, child_bounds) in children.iter().zip(child_bounds) {
        child.paint(child_bounds, context);
    }
}

pub fn border_box(node: &LayoutNode, parent_origin: Point) -> Option<Rect> {
    let box_model = node.layout_boxes.iter().next()?;

    let rect = box_model.border_box;

    Some(Rect::new(
        parent_origin.x + rect.x,
        parent_origin.y + rect.y,
        rect.width,
        rect.height,
    ))
}

struct EmptyView;

impl View for EmptyView {
    fn paint(&self, _bounds: Rect, _context: &mut PaintContext<'_>) {}
}

#[allow(unused)]
pub(crate) fn dispatch_child_event(
    child: &StackChild,
    bounds: Rect,
    event: &ViewEvent,
    context: &mut EventContext,
) -> EventResult {
    if !event.requires_broadcast() && !event.is_inside(bounds) {
        return EventResult::Ignored;
    }

    child.handle_event(bounds, event, context)
}

#[allow(unused)]
pub(crate) fn dispatch_children_in_order<'a>(
    children: impl IntoIterator<Item = (&'a StackChild, Rect)>,
    event: &ViewEvent,
    context: &mut EventContext,
) -> EventResult {
    let broadcast = event.requires_broadcast();

    let mut result = EventResult::Ignored;

    for (child, bounds) in children {
        let child_result = dispatch_child_event(child, bounds, event, context);

        result = result.merge(child_result);

        if !broadcast && child_result.is_consumed() {
            break;
        }
    }

    result
}

pub(crate) fn layout_stack(
    direction: StackDirection,
    children: &[StackChild],
    bounds: Rect,
    gap: StackGap,
    alignment: StackAlignment,
    distribution: StackDistribution,
    theme: &Theme,
    measure_context: &mut MeasureContext<'_>,
) -> Vec<Rect> {
    if children.is_empty() || bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
        return Vec::new();
    }

    #[derive(Clone, Copy)]
    struct LayoutItem {
        main_size: f32,
        cross_size: f32,

        flex_grow: f32,
        flex_shrink: f32,

        cross_auto: bool,
        divider: bool,
    }

    fn finite_non_negative(value: f32) -> f32 {
        if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        }
    }

    let available_main = match direction {
        StackDirection::Horizontal => bounds.size.width,

        StackDirection::Vertical => bounds.size.height,
    }
    .max(0.0);

    let available_cross = match direction {
        StackDirection::Horizontal => bounds.size.height,

        StackDirection::Vertical => bounds.size.width,
    }
    .max(0.0);

    let resolved_gap = finite_non_negative(gap.resolve(&theme.spacing));

    let child_constraints = Constraints::loose(bounds.size);

    let mut items = Vec::with_capacity(children.len());

    for child in children {
        let measured = child.measure(child_constraints, measure_context);

        let (width, height, divider) = match child.kind {
            StackChildKind::Normal => {
                let width = match child.width {
                    LayoutLength::Auto => finite_non_negative(measured.width),

                    LayoutLength::Fixed(value) => sanitize_length(value),
                };

                let height = match child.height {
                    LayoutLength::Auto => finite_non_negative(measured.height),

                    LayoutLength::Fixed(value) => sanitize_length(value),
                };

                (width, height, false)
            }

            StackChildKind::Divider { thickness } => {
                let thickness = finite_non_negative(thickness.resolve(&theme.divider));

                match direction {
                    StackDirection::Horizontal => (thickness, available_cross, true),

                    StackDirection::Vertical => (available_cross, thickness, true),
                }
            }
        };

        let (main_size, cross_size, cross_auto) = match direction {
            StackDirection::Horizontal => {
                (width, height, matches!(child.height, LayoutLength::Auto))
            }

            StackDirection::Vertical => (height, width, matches!(child.width, LayoutLength::Auto)),
        };

        items.push(LayoutItem {
            main_size: finite_non_negative(main_size),

            cross_size: finite_non_negative(cross_size),

            flex_grow: if divider {
                0.0
            } else {
                finite_non_negative(child.flex_grow)
            },

            flex_shrink: if divider {
                0.0
            } else {
                finite_non_negative(child.flex_shrink)
            },

            cross_auto,
            divider,
        });
    }

    let gap_count = items.len().saturating_sub(1);
    let total_gap = resolved_gap * gap_count as f32;
    let base_main_size = items.iter().map(|item| item.main_size).sum::<f32>();
    let free_space = available_main - base_main_size - total_gap;

    if free_space > 0.0 {
        let total_grow = items.iter().map(|item| item.flex_grow).sum::<f32>();

        if total_grow > 0.0 {
            for item in &mut items {
                if item.flex_grow <= 0.0 {
                    continue;
                }

                item.main_size += free_space * (item.flex_grow / total_grow);
            }
        }
    } else if free_space < 0.0 {
        let overflow = -free_space;

        let total_shrink_weight = items
            .iter()
            .map(|item| item.flex_shrink * item.main_size)
            .sum::<f32>();

        if total_shrink_weight > 0.0 {
            for item in &mut items {
                let weight = item.flex_shrink * item.main_size;

                if weight <= 0.0 {
                    continue;
                }

                let reduction = overflow * (weight / total_shrink_weight);

                item.main_size = (item.main_size - reduction).max(0.0);
            }
        }
    }

    let occupied_main = items.iter().map(|item| item.main_size).sum::<f32>() + total_gap;

    let remaining_space = (available_main - occupied_main).max(0.0);

    let mut leading_space = 0.0;
    let mut actual_gap = resolved_gap;

    match distribution {
        StackDistribution::Start => {}

        StackDistribution::Center => {
            leading_space = remaining_space / 2.0;
        }

        StackDistribution::End => {
            leading_space = remaining_space;
        }

        StackDistribution::SpaceBetween => {
            if gap_count > 0 {
                actual_gap += remaining_space / gap_count as f32;
            }
        }

        StackDistribution::SpaceAround => {
            let spacing = remaining_space / items.len() as f32;

            leading_space = spacing / 2.0;

            actual_gap += spacing;
        }

        StackDistribution::SpaceEvenly => {
            let spacing = remaining_space / (items.len() as f32 + 1.0);

            leading_space = spacing;
            actual_gap += spacing;
        }
    }

    let mut cursor = match direction {
        StackDirection::Horizontal => bounds.origin.x,

        StackDirection::Vertical => bounds.origin.y,
    } + leading_space;

    let mut result = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let cross_size =
            if item.divider || (alignment == StackAlignment::Stretch && item.cross_auto) {
                available_cross
            } else {
                item.cross_size.min(available_cross)
            };

        let cross_offset = match alignment {
            StackAlignment::Start | StackAlignment::Stretch => 0.0,

            StackAlignment::Center => (available_cross - cross_size) / 2.0,

            StackAlignment::End => available_cross - cross_size,
        }
        .max(0.0);

        let child_bounds = match direction {
            StackDirection::Horizontal => Rect::new(
                cursor,
                bounds.origin.y + cross_offset,
                item.main_size,
                cross_size,
            ),

            StackDirection::Vertical => Rect::new(
                bounds.origin.x + cross_offset,
                cursor,
                cross_size,
                item.main_size,
            ),
        };

        result.push(child_bounds);

        cursor += item.main_size;

        if index + 1 < items.len() {
            cursor += actual_gap;
        }
    }

    result
}

pub(crate) fn handle_stack_event(
    direction: StackDirection,
    children: &[StackChild],
    bounds: Rect,
    gap: StackGap,
    alignment: StackAlignment,
    distribution: StackDistribution,
    event: &ViewEvent,
    context: &mut EventContext<'_>,
) -> EventResult {
    let child_bounds = {
        let theme = context.theme;

        let typography = context.typography;

        let text_measurer = &mut *context.text_measurer;

        let mut measure_context = MeasureContext {
            theme,
            typography,
            text_measurer,
        };

        layout_stack(
            direction,
            children,
            bounds,
            gap,
            alignment,
            distribution,
            theme,
            &mut measure_context,
        )
    };

    if event.requires_broadcast() {
        let mut result = EventResult::Ignored;

        for (child, child_bounds) in children.iter().zip(child_bounds.iter().copied()) {
            result = result.merge(child.handle_event(child_bounds, event, context));
        }

        return result;
    }

    let Some(position) = event.position() else {
        return EventResult::Ignored;
    };

    for index in (0..children.len().min(child_bounds.len())).rev() {
        let bounds = child_bounds[index];

        if !bounds.contains(position) {
            continue;
        }

        let result = children[index].handle_event(bounds, event, context);

        if result.is_consumed() {
            return result;
        }
    }

    EventResult::Ignored
}

fn resolve_axis_constraints(length: LayoutLength, minimum: f32, maximum: f32) -> (f32, f32) {
    match length {
        LayoutLength::Auto => (minimum.max(0.0), maximum.max(minimum)),

        LayoutLength::Fixed(value) => {
            let value = sanitize_length(value).clamp(minimum.max(0.0), maximum.max(minimum));

            (value, value)
        }
    }
}

fn sanitize_length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn measured_ui_length(length: LayoutLength, measured: f32) -> Length {
    match length {
        LayoutLength::Fixed(value) => Length::Px(sanitize_length(value)),

        LayoutLength::Auto => {
            let measured = sanitize_length(measured);

            if measured > 0.0 {
                Length::Px(measured)
            } else {
                // 固有サイズを持たないやつはAuto
                Length::Auto
            }
        }
    }
}
