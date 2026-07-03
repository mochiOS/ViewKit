use crate::draw_command::DisplayList;
use crate::event::{EventContext, EventResult, ViewEvent};
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
        Size::new(
            size.width.clamp(self.minimum.width, self.maximum.width),
            size.height.clamp(self.minimum.height, self.maximum.height),
        )
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
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RedrawSchedule {
    deadline: Option<Instant>,
}

impl RedrawSchedule {
    pub const fn new() -> Self {
        Self { deadline: None }
    }

    pub const fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn request_at(&mut self, deadline: Instant) {
        match self.deadline {
            Some(current) if current <= deadline => {}

            _ => {
                self.deadline = Some(deadline);
            }
        }
    }

    pub fn take(&mut self) -> Option<Instant> {
        self.deadline.take()
    }

    pub fn clear(&mut self) {
        self.deadline = None;
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
