//! Standard settings-page composition.

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{
    IntoStackChild, IntoStackChildren, StackAlignment, StackChild, StackDirection,
    StackDistribution, StackGap, handle_stack_event, measure_stack, paint_stack,
};
use crate::typography::{TextAlignment, TextRole};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{FormSections, PageHeader, Text, TextTone, VStack};

/// A page header followed by a centered, standard-width settings form.
pub struct SettingsPage<Content> {
    title: String,
    subtitle: Option<String>,
    content: Content,
}

impl<Content> SettingsPage<Content> {
    pub fn new(title: impl Into<String>, content: Content) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            content,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        let subtitle = subtitle.into();
        self.subtitle = (!subtitle.is_empty()).then_some(subtitle);
        self
    }

    fn header(&self) -> PageHeader {
        let header = PageHeader::new(self.title.clone());
        match &self.subtitle {
            Some(subtitle) => header.subtitle(subtitle.clone()),
            None => header,
        }
    }

    fn form_bounds(bounds: Rect, header_height: f32, gap: f32, form_width: f32) -> Rect {
        let width = bounds.size.width.min(form_width).max(0.0);
        Rect::new(
            bounds.origin.x + (bounds.size.width - width).max(0.0) / 2.0,
            bounds.origin.y + header_height + gap,
            width,
            (bounds.size.height - header_height - gap).max(0.0),
        )
    }
}

impl SettingsPage<FormSections> {
    /// Starts a standard settings form. Sections can then be appended without
    /// manually composing `FormSections` or repeating the section spacing.
    ///
    /// ```ignore
    /// SettingsPage::form("General")
    ///     .subtitle("Device, language, region, date, and system information")
    ///     .section(
    ///         SettingsSection::new("Language & Region")
    ///             .row(SettingsRow::new("Language", language_picker)
    ///                 .description("Primary system language")),
    ///     )
    /// ```
    pub fn form(title: impl Into<String>) -> Self {
        Self::new(title, FormSections::new())
    }

    pub fn section<Section: IntoStackChildren>(mut self, section: Section) -> Self {
        self.content = std::mem::take(&mut self.content).section(section);
        self
    }

    pub fn sections<Section: IntoStackChildren>(
        mut self,
        sections: impl IntoIterator<Item = Section>,
    ) -> Self {
        for section in sections {
            self.content = std::mem::take(&mut self.content).section(section);
        }
        self
    }
}

impl<Content: View> View for SettingsPage<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let header = self.header().measure(constraints, context);
        let form = self.content.measure(
            Constraints::loose(Size::new(
                constraints.maximum.width.min(context.theme.layout.form_width),
                constraints.maximum.height,
            )),
            context,
        );
        constraints.constrain(Size::new(
            header.width.max(form.width),
            header.height + context.theme.layout.section_gap + form.height,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let header_height = {
            let mut measure = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };
            self.header()
                .measure(Constraints::loose(bounds.size), &mut measure)
                .height
        };
        self.header().paint(
            Rect::new(bounds.origin.x, bounds.origin.y, bounds.size.width, header_height),
            context,
        );
        self.content.paint(
            Self::form_bounds(
                bounds,
                header_height,
                context.theme.layout.section_gap,
                context.theme.layout.form_width,
            ),
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let header_height = {
            let mut measure = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };
            self.header()
                .measure(Constraints::loose(bounds.size), &mut measure)
                .height
        };
        self.content.handle_event(
            Self::form_bounds(
                bounds,
                header_height,
                context.theme.layout.section_gap,
                context.theme.layout.form_width,
            ),
            event,
            context,
        )
    }
}

/// A titled group of settings rows. Rows are separated by whitespace, never
/// by automatically inserted dividers.
pub struct SettingsSection {
    title: String,
    rows: Vec<StackChild>,
}

impl SettingsSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    pub fn row<Row: IntoStackChild>(mut self, row: Row) -> Self {
        self.rows.push(row.into_stack_child());
        self
    }

    pub fn rows<Row: IntoStackChild>(
        mut self,
        rows: impl IntoIterator<Item = Row>,
    ) -> Self {
        self.rows
            .extend(rows.into_iter().map(IntoStackChild::into_stack_child));
        self
    }

    fn title(&self) -> Text {
        Text::styled(self.title.clone(), TextRole::Label)
            .weight(600)
            .tone(TextTone::Secondary)
    }
}

impl View for SettingsSection {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let title = self.title().measure(constraints, context);
        let rows = measure_stack(
            StackDirection::Vertical,
            &self.rows,
            StackGap::None,
            constraints,
            context,
        );
        constraints.constrain(Size::new(
            title.width.max(rows.width),
            title.height + context.theme.spacing.small + rows.height,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let title_height = {
            let mut measure = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };
            self.title()
                .measure(Constraints::loose(bounds.size), &mut measure)
                .height
        };
        self.title().paint(
            Rect::new(bounds.origin.x, bounds.origin.y, bounds.size.width, title_height),
            context,
        );
        let rows_y = bounds.origin.y + title_height + context.theme.spacing.small;
        paint_stack(
            StackDirection::Vertical,
            &self.rows,
            Rect::new(
                bounds.origin.x,
                rows_y,
                bounds.size.width,
                (bounds.origin.y + bounds.size.height - rows_y).max(0.0),
            ),
            StackGap::None,
            StackAlignment::Stretch,
            StackDistribution::Start,
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let title_height = {
            let mut measure = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };
            self.title()
                .measure(Constraints::loose(bounds.size), &mut measure)
                .height
        };
        let rows_y = bounds.origin.y + title_height + context.theme.spacing.small;
        handle_stack_event(
            StackDirection::Vertical,
            &self.rows,
            Rect::new(
                bounds.origin.x,
                rows_y,
                bounds.size.width,
                (bounds.origin.y + bounds.size.height - rows_y).max(0.0),
            ),
            StackGap::None,
            StackAlignment::Stretch,
            StackDistribution::Start,
            event,
            context,
        )
    }
}

/// A standard label/description/control row for a settings form.
pub struct SettingsRow {
    title: String,
    description: Option<String>,
    control: StackChild,
}

impl SettingsRow {
    pub fn new<Control: IntoStackChild>(title: impl Into<String>, control: Control) -> Self {
        Self {
            title: title.into(),
            description: None,
            control: control.into_stack_child(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        let description = description.into();
        self.description = (!description.is_empty()).then_some(description);
        self
    }

    fn labels(&self) -> VStack {
        let mut labels = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(Text::styled(self.title.clone(), TextRole::Body).weight(600));
        if let Some(description) = &self.description {
            labels = labels.child(
                Text::styled(description.clone(), TextRole::Caption).tone(TextTone::Secondary),
            );
        }
        labels
    }

    fn layout(
        &self,
        bounds: Rect,
        control_size: Size,
        horizontal_padding: f32,
        gap: f32,
    ) -> (Rect, Rect) {
        let control_width = control_size.width.min(bounds.size.width).max(0.0);
        let control_height = control_size.height.min(bounds.size.height).max(0.0);
        let control = Rect::new(
            bounds.origin.x + bounds.size.width - control_width,
            bounds.origin.y + (bounds.size.height - control_height).max(0.0) / 2.0,
            control_width,
            control_height,
        );
        let labels = Rect::new(
            bounds.origin.x + horizontal_padding,
            bounds.origin.y,
            (control.origin.x - gap - bounds.origin.x - horizontal_padding).max(0.0),
            bounds.size.height,
        );
        (labels, control)
    }
}

impl View for SettingsRow {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let control = self.control.measure(constraints, context);
        let labels = self.labels().measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - control.width - context.theme.spacing.large).max(0.0),
                constraints.maximum.height,
            )),
            context,
        );
        constraints.constrain(Size::new(
            constraints.maximum.width,
            labels.height.max(control.height) + context.theme.spacing.medium,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let control_size = {
            let mut measure = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };
            self.control
                .measure(Constraints::loose(bounds.size), &mut measure)
        };
        let (labels, control) = self.layout(
            bounds,
            control_size,
            context.theme.spacing.small,
            context.theme.spacing.large,
        );
        self.labels().paint(labels, context);
        self.control.paint(control, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let control_size = {
            let mut measure = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };
            self.control
                .measure(Constraints::loose(bounds.size), &mut measure)
        };
        let (_, control) = self.layout(
            bounds,
            control_size,
            context.theme.spacing.small,
            context.theme.spacing.large,
        );
        self.control.handle_event(control, event, context)
    }
}

/// Convenience constructor for a read-only value aligned like a control.
pub fn settings_value(
    title: impl Into<String>,
    description: impl Into<String>,
    value: impl Into<String>,
) -> SettingsRow {
    SettingsRow::new(
        title,
        Text::styled(value.into(), TextRole::Body)
            .alignment(TextAlignment::End)
            .tone(TextTone::Secondary),
    )
    .description(description)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Recorder(Rc<Cell<Option<Rect>>>);

    impl View for Recorder {
        fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
            constraints.constrain(Size::new(264.0, 32.0))
        }

        fn paint(&self, bounds: Rect, _context: &mut PaintContext<'_>) {
            self.0.set(Some(bounds));
        }
    }

    #[test]
    fn settings_page_centers_a_480px_form_inside_720px_content() {
        let recorded = Rc::new(Cell::new(None));
        let page = SettingsPage::new("General", Recorder(recorded.clone()))
            .subtitle("Device, language, region, date, and system information");
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        page.paint(Rect::new(0.0, 0.0, 720.0, 600.0), &mut context);

        let form = recorded.get().expect("form bounds");
        assert_eq!(form.origin.x, 120.0);
        assert_eq!(form.size.width, 480.0);
    }
}
