//! Standard settings-page composition.

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{
    IntoStackChild, IntoStackChildren, StackAlignment, StackChild, StackDirection,
    StackDistribution, StackGap, handle_stack_event, layout_stack, measure_stack, paint_stack,
};
use crate::typography::{TextAlignment, TextRole};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{FormSections, PageHeader, Rectangle, RectangleColor, Text, TextTone, VStack};
use crate::theme::CornerRadius;

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
                constraints
                    .maximum
                    .width
                    .min(context.theme.layout.form_width),
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
            Rect::new(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                header_height,
            ),
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

/// A titled group of settings rows. The default presentation uses whitespace;
/// `grouped()` opts into one inset surface with separators.
pub struct SettingsSection {
    title: String,
    rows: Vec<StackChild>,
    grouped: bool,
}

impl SettingsSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
            grouped: false,
        }
    }

    /// Places related rows on one inset surface with quiet separators.
    /// Existing ungrouped forms retain their original presentation.
    pub fn grouped(mut self) -> Self {
        self.grouped = true;
        self
    }

    pub fn row<Row: IntoStackChild>(mut self, row: Row) -> Self {
        self.rows.push(row.into_stack_child());
        self
    }

    pub fn rows<Row: IntoStackChild>(mut self, rows: impl IntoIterator<Item = Row>) -> Self {
        self.rows
            .extend(rows.into_iter().map(IntoStackChild::into_stack_child));
        self
    }

    fn title(&self) -> Text {
        Text::styled(self.title.clone(), TextRole::Label)
            .weight(600)
            .tone(TextTone::Secondary)
    }

    fn inset(&self, theme: &crate::theme::Theme) -> f32 {
        if self.grouped {
            theme.spacing.large
        } else {
            0.0
        }
    }
}

impl View for SettingsSection {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let inset = self.inset(context.theme);
        let inner = Constraints::loose(Size::new(
            (constraints.maximum.width - inset * 2.0).max(0.0),
            constraints.maximum.height,
        ));
        let title = self.title().measure(inner, context);
        let rows = measure_stack(
            StackDirection::Vertical,
            &self.rows,
            StackGap::None,
            inner,
            context,
        );
        constraints.constrain(Size::new(
            title.width.max(rows.width) + inset * 2.0,
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
                .measure(
                    Constraints::loose(Size::new(
                        (bounds.size.width - self.inset(context.theme) * 2.0).max(0.0),
                        bounds.size.height,
                    )),
                    &mut measure,
                )
                .height
        };
        self.title().paint(
            Rect::new(
                bounds.origin.x + self.inset(context.theme),
                bounds.origin.y,
                (bounds.size.width - self.inset(context.theme) * 2.0).max(0.0),
                title_height,
            ),
            context,
        );
        let rows_y = bounds.origin.y + title_height + context.theme.spacing.small;
        let inset = self.inset(context.theme);
        if self.grouped {
            Rectangle::new()
                .color(RectangleColor::Surface)
                .radius(CornerRadius::Medium)
                .paint(
                    Rect::new(
                        bounds.origin.x,
                        rows_y,
                        bounds.size.width,
                        (bounds.origin.y + bounds.size.height - rows_y).max(0.0),
                    ),
                    context,
                );
        }
        let rows_bounds = Rect::new(
            bounds.origin.x + inset,
            rows_y,
            (bounds.size.width - inset * 2.0).max(0.0),
            (bounds.origin.y + bounds.size.height - rows_y).max(0.0),
        );
        if self.grouped {
            let regions = {
                let mut measure = MeasureContext {
                    theme: context.theme,
                    typography: context.typography,
                    text_measurer: &mut *context.text_measurer,
                };
                layout_stack(
                    StackDirection::Vertical,
                    &self.rows,
                    rows_bounds,
                    StackGap::None,
                    StackAlignment::Stretch,
                    StackDistribution::Start,
                    context.theme,
                    &mut measure,
                )
            };
            for (index, (row, region)) in self.rows.iter().zip(regions).enumerate() {
                if index > 0 {
                    context
                        .display_list
                        .push(crate::draw_command::DrawCommand::FillRect {
                            rect: Rect::new(
                                rows_bounds.origin.x,
                                region.origin.y,
                                rows_bounds.size.width,
                                context.theme.divider.thickness,
                            ),
                            color: context.theme.colors.border,
                        });
                }
                row.paint(region, context);
            }
            return;
        }
        paint_stack(
            StackDirection::Vertical,
            &self.rows,
            rows_bounds,
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
                .measure(
                    Constraints::loose(Size::new(
                        (bounds.size.width - self.inset(context.theme) * 2.0).max(0.0),
                        bounds.size.height,
                    )),
                    &mut measure,
                )
                .height
        };
        let rows_y = bounds.origin.y + title_height + context.theme.spacing.small;
        let inset = self.inset(context.theme);
        handle_stack_event(
            StackDirection::Vertical,
            &self.rows,
            Rect::new(
                bounds.origin.x + inset,
                rows_y,
                (bounds.size.width - inset * 2.0).max(0.0),
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
    compact: bool,
}

impl SettingsRow {
    pub fn new<Control: IntoStackChild>(title: impl Into<String>, control: Control) -> Self {
        Self {
            title: title.into(),
            description: None,
            control: control.into_stack_child(),
            compact: false,
        }
    }

    /// Uses native control typography and vertically centered labels. On narrow
    /// windows the control moves below the label instead of crushing its text.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
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
            .child(
                Text::styled(
                    self.title.clone(),
                    if self.compact {
                        TextRole::Label
                    } else {
                        TextRole::Body
                    },
                )
                .weight(if self.compact { 500 } else { 600 }),
            );
        if let Some(description) = &self.description {
            labels = labels.child(
                Text::styled(description.clone(), TextRole::Caption).tone(TextTone::Secondary),
            );
        }
        labels
    }

    fn compact_layout(&self, width: f32, context: &mut MeasureContext<'_>) -> (Size, Rect, Rect) {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            context.theme.layout.form_width
        };
        let padding = context.theme.spacing.small;
        let gap = context.theme.spacing.large;
        let control = self
            .control
            .measure(Constraints::loose(Size::new(width, f32::INFINITY)), context);
        let stacked = width < control.width + context.theme.layout.compact_form_control_width + gap;
        let label_width = if stacked {
            width
        } else {
            (width - control.width - gap).max(0.0)
        };
        let labels = self.labels().measure(
            Constraints::loose(Size::new(label_width, f32::INFINITY)),
            context,
        );
        if stacked {
            let control_y = padding + labels.height + padding;
            let height = control_y + control.height + padding;
            (
                Size::new(width, height),
                Rect::new(0.0, padding, width, labels.height),
                Rect::new(
                    width - control.width,
                    control_y,
                    control.width,
                    control.height,
                ),
            )
        } else {
            let height = labels
                .height
                .max(control.height)
                .max(context.theme.controls.height)
                + padding * 2.0;
            (
                Size::new(width, height),
                Rect::new(
                    0.0,
                    (height - labels.height) / 2.0,
                    label_width,
                    labels.height,
                ),
                Rect::new(
                    width - control.width,
                    (height - control.height) / 2.0,
                    control.width,
                    control.height,
                ),
            )
        }
    }

    fn compact_regions(&self, bounds: Rect, context: &mut MeasureContext<'_>) -> (Rect, Rect) {
        let (_, mut labels, mut control) = self.compact_layout(bounds.size.width, context);
        for rect in [&mut labels, &mut control] {
            rect.origin.x += bounds.origin.x;
            rect.origin.y += bounds.origin.y;
        }
        (labels, control)
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
        if self.compact {
            return constraints
                .constrain(self.compact_layout(constraints.maximum.width, context).0);
        }
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
        if self.compact {
            let (labels, control) = self.compact_regions(
                bounds,
                &mut MeasureContext {
                    theme: context.theme,
                    typography: context.typography,
                    text_measurer: &mut *context.text_measurer,
                },
            );
            self.labels().paint(labels, context);
            self.control.paint(control, context);
            return;
        }
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
        if self.compact {
            let (_, control) = self.compact_regions(
                bounds,
                &mut MeasureContext {
                    theme: context.theme,
                    typography: context.typography,
                    text_measurer: &mut *context.text_measurer,
                },
            );
            return self.control.handle_event(control, event, context);
        }
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
    use crate::layout::ViewExt;
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
    fn compact_row_keeps_labels_clear_and_its_control_clickable_after_reflow() {
        use crate::components::Button;
        use crate::geometry::Point;
        use crate::platform::PointerButton;
        for width in [280.0, 440.0, 640.0] {
            let calls = Rc::new(Cell::new(0));
            let click = calls.clone();
            let row = SettingsRow::new(
                "Language",
                Button::new("Japanese")
                    .on_click(move || click.set(click.get() + 1))
                    .frame(264.0, 30.0),
            )
            .description("Primary system language")
            .compact();
            let mut measurer = TextMeasurer::new();
            let mut measure = MeasureContext {
                theme: &Theme::LIGHT,
                typography: &Typography::DEFAULT,
                text_measurer: &mut measurer,
            };
            let (size, labels, control) = row.compact_layout(width, &mut measure);
            assert!(
                labels.origin.x + labels.size.width <= control.origin.x
                    || labels.origin.y + labels.size.height <= control.origin.y
            );
            assert!(control.origin.x >= 0.0);
            assert!(control.origin.x + control.size.width <= size.width);
            assert!(control.origin.y + control.size.height <= size.height);
            let bounds = Rect::new(0.0, 0.0, size.width, size.height);
            let position = Point::new(
                control.origin.x + control.size.width / 2.0,
                control.origin.y + control.size.height / 2.0,
            );
            let mut context = EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut measurer);
            row.handle_event(
                bounds,
                &ViewEvent::PointerPressed {
                    position,
                    button: PointerButton::Primary,
                },
                &mut context,
            );
            row.handle_event(
                bounds,
                &ViewEvent::PointerReleased {
                    position,
                    button: PointerButton::Primary,
                },
                &mut context,
            );
            assert_eq!(calls.get(), 1, "control at width {width}");
        }
    }

    #[test]
    fn grouped_section_insets_both_rows_and_preserves_their_natural_height() {
        let first = Rc::new(Cell::new(None));
        let second = Rc::new(Cell::new(None));
        let section = SettingsSection::new("Language & Region")
            .grouped()
            .row(Recorder(first.clone()))
            .row(Recorder(second.clone()));
        let mut list = DisplayList::new();
        let mut measurer = TextMeasurer::new();
        let size = section.measure(
            Constraints::loose(Size::new(500.0, f32::INFINITY)),
            &mut MeasureContext {
                theme: &Theme::LIGHT,
                typography: &Typography::DEFAULT,
                text_measurer: &mut measurer,
            },
        );
        let mut context = PaintContext::new(
            &mut list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut measurer,
        );
        section.paint(Rect::new(20.0, 10.0, 500.0, size.height), &mut context);
        let first = first.get().unwrap();
        let second = second.get().unwrap();
        assert_eq!(first.origin.x, 36.0);
        assert_eq!(first.size.width, 468.0);
        assert_eq!(second.origin.y, first.origin.y + first.size.height);
        assert_eq!(first.size.height, 32.0);
        assert_eq!(second.size.height, 32.0);
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
