use crate::draw_command::DisplayList;
use crate::event::{EventContext, EventResult, RedrawRequest, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    pub minimum: Size,
    pub maximum: Size,
}

impl Constraints {
    pub fn new(minimum: Size, maximum: Size) -> Self {
        Self { minimum, maximum }
    }

    pub fn loose(maximum: Size) -> Self {
        Self {
            minimum: Size::new(0.0, 0.0),
            maximum,
        }
    }

    pub fn constrain(self, size: Size) -> Size {
        let minimum_width = sanitize_minimum(self.minimum.width);
        let minimum_height = sanitize_minimum(self.minimum.height);
        let maximum_width = sanitize_maximum(self.maximum.width).max(minimum_width);
        let maximum_height = sanitize_maximum(self.maximum.height).max(minimum_height);

        Size::new(
            sanitize_minimum(size.width).clamp(minimum_width, maximum_width),
            sanitize_minimum(size.height).clamp(minimum_height, maximum_height),
        )
    }
}

fn sanitize_minimum(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn sanitize_maximum(value: f32) -> f32 {
    if value == f32::INFINITY {
        f32::INFINITY
    } else if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub struct MeasureContext<'a> {
    pub theme: &'a Theme,
    pub typography: &'a Typography,
    pub text_measurer: &'a mut TextMeasurer,
}

pub struct PaintContext<'a> {
    pub display_list: &'a mut DisplayList,
    pub theme: &'a Theme,
    pub typography: &'a Typography,
    pub text_measurer: &'a mut TextMeasurer,
    redraw_schedule: Option<&'a mut RedrawSchedule>,
    inherited_corner_radii: Vec<f32>,
}

impl<'a> PaintContext<'a> {
    pub fn new(
        display_list: &'a mut DisplayList,
        theme: &'a Theme,
        typography: &'a Typography,
        text_measurer: &'a mut TextMeasurer,
    ) -> Self {
        Self {
            display_list,
            theme,
            typography,
            text_measurer,

            redraw_schedule: None,
            inherited_corner_radii: Vec::new(),
        }
    }

    pub fn with_redraw_schedule(mut self, redraw_schedule: &'a mut RedrawSchedule) -> Self {
        self.redraw_schedule = Some(redraw_schedule);
        self
    }

    pub fn request_redraw_at(&mut self, deadline: Instant) {
        let Some(schedule) = self.redraw_schedule.as_deref_mut() else {
            return;
        };

        schedule.request_at(deadline);
    }

    pub fn request_redraw_in_at(&mut self, bounds: Rect, deadline: Instant) {
        if bounds.is_empty() {
            return;
        }

        let Some(schedule) = self.redraw_schedule.as_deref_mut() else {
            return;
        };

        schedule.request_in_at(bounds, deadline);
    }

    pub(crate) fn inherited_corner_radius(&self) -> Option<f32> {
        self.inherited_corner_radii.last().copied()
    }

    pub(crate) fn push_corner_radius(&mut self, radius: f32) {
        self.inherited_corner_radii.push(radius.max(0.0));
    }

    pub(crate) fn pop_corner_radius(&mut self) {
        self.inherited_corner_radii.pop();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RedrawSchedule {
    deadline: Option<Instant>,
    request: RedrawRequest,
}

impl RedrawSchedule {
    pub const fn new() -> Self {
        Self {
            deadline: None,
            request: RedrawRequest::None,
        }
    }

    pub const fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn request_at(&mut self, deadline: Instant) {
        self.request(deadline, RedrawRequest::Full);
    }

    pub fn request_in_at(&mut self, bounds: Rect, deadline: Instant) {
        if bounds.is_empty() {
            return;
        }

        self.request(deadline, RedrawRequest::Region(bounds));
    }

    fn request(&mut self, deadline: Instant, request: RedrawRequest) {
        match self.deadline {
            Some(current) if current <= deadline => {}

            _ => {
                self.deadline = Some(deadline);
            }
        }

        self.request = self.request.merge(request);
    }

    pub(crate) fn take_due(&mut self, now: Instant) -> RedrawRequest {
        if self.deadline.is_none_or(|deadline| deadline > now) {
            return RedrawRequest::None;
        }

        self.deadline = None;
        std::mem::take(&mut self.request)
    }

    pub fn clear(&mut self) {
        self.deadline = None;
        self.request = RedrawRequest::None;
    }
}

pub trait View {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(0.0, 0.0))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>);

    fn handle_event(
        &self,
        _bounds: Rect,
        _event: &ViewEvent,
        _context: &mut EventContext<'_>,
    ) -> EventResult {
        EventResult::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn infinite_maximum_keeps_intrinsic_size_unbounded() {
        let constraints = Constraints::loose(Size::new(320.0, f32::INFINITY));

        assert_eq!(
            constraints.constrain(Size::new(280.0, 900.0)),
            Size::new(280.0, 900.0)
        );
    }

    #[test]
    fn scheduled_regions_merge_and_remain_pending_until_due() {
        let now = Instant::now();
        let deadline = now + Duration::from_millis(20);
        let first = Rect::new(10.0, 20.0, 30.0, 40.0);
        let second = Rect::new(35.0, 50.0, 20.0, 30.0);
        let mut schedule = RedrawSchedule::new();

        schedule.request_in_at(first, deadline);
        schedule.request_in_at(second, deadline + Duration::from_millis(10));

        assert_eq!(schedule.deadline(), Some(deadline));
        assert_eq!(schedule.take_due(now), RedrawRequest::None);
        assert_eq!(
            schedule.take_due(deadline),
            RedrawRequest::Region(first.union(second))
        );
        assert_eq!(schedule.deadline(), None);
    }

    #[test]
    fn scheduled_full_redraw_overrides_regions() {
        let deadline = Instant::now();
        let mut schedule = RedrawSchedule::new();

        schedule.request_in_at(Rect::new(1.0, 2.0, 3.0, 4.0), deadline);
        schedule.request_at(deadline);

        assert_eq!(schedule.take_due(deadline), RedrawRequest::Full);
    }
}
