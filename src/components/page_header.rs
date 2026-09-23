//! Standard heading for a scrollable application page.

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap};
use crate::typography::TextRole;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Text, TextTone, VStack};

/// Establishes the visual hierarchy of a page independently of window chrome.
///
/// Window and toolbar titles describe navigation. `PageHeader` describes the
/// content currently visible below them, so applications should not substitute
/// a small caption for the page title.
pub struct PageHeader {
    title: String,
    subtitle: Option<String>,
}

impl PageHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        let subtitle = subtitle.into();
        self.subtitle = (!subtitle.is_empty()).then_some(subtitle);
        self
    }

    fn content(&self) -> VStack {
        let mut content = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::ExtraSmall)
            .child(Text::styled(self.title.clone(), TextRole::TitleLarge));
        if let Some(subtitle) = &self.subtitle {
            content = content.child(
                Text::styled(subtitle.clone(), TextRole::Body).tone(TextTone::Secondary),
            );
        }
        content
    }
}

impl View for PageHeader {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content().measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.content().paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.content().handle_event(bounds, event, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn subtitle_adds_a_distinct_body_line_without_fixed_height() {
        let mut text_measurer = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &Theme::LIGHT,
            typography: &Typography::DEFAULT,
            text_measurer: &mut text_measurer,
        };
        let without_subtitle = PageHeader::new("General").measure(
            Constraints::loose(Size::new(640.0, 200.0)),
            &mut context,
        );
        let with_subtitle = PageHeader::new("General")
            .subtitle("Device, language, region, date, and system information")
            .measure(
                Constraints::loose(Size::new(640.0, 200.0)),
                &mut context,
            );

        assert!(with_subtitle.height > without_subtitle.height);
        assert!(with_subtitle.width <= 640.0);
    }
}
