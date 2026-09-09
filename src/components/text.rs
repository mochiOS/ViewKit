//! 文字列を描画するTextを定義

use cosmic_text::{Attrs, Buffer, Metrics, Shaping, Weight};

use crate::draw_command::{DrawCommand, TextCommand};
use crate::font::{DEFAULT_MONOSPACE_FONT_FAMILY, DEFAULT_UI_FONT_FAMILY, resolve_font_family};
use crate::geometry::{Rect, Size};
use crate::runtime::{IntoViewNode, TextNode, ViewNode, ViewNodeContext, ViewNodeKind};
use crate::theme::{Color, Theme};
use crate::typography::{
    FontFamily, FontWeight, TextAlignment, TextMeasurer, TextRole, Typography,
};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

pub struct Text {
    value: String,

    role: TextRole,
    font_family: Option<String>,
    font_size: Option<f32>,
    line_height: Option<f32>,
    weight: Option<u16>,

    alignment: TextAlignment,

    color: Option<Color>,
    tone: TextTone,
    cache_layout: bool,
}

impl Text {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),

            role: TextRole::Body,
            font_family: None,
            font_size: None,
            line_height: None,
            weight: None,

            alignment: TextAlignment::Start,

            color: None,
            tone: TextTone::Primary,
            cache_layout: true,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn styled(value: impl Into<String>, role: TextRole) -> Self {
        Self::new(value).style(role)
    }

    pub fn body(value: impl Into<String>) -> Self {
        Self::styled(value, TextRole::Body)
    }

    pub fn body_emphasized(value: impl Into<String>) -> Self {
        Self::new(value).weight(FontWeight::MEDIUM.0)
    }

    pub fn label(value: impl Into<String>) -> Self {
        Self::styled(value, TextRole::Label)
    }

    pub fn caption(value: impl Into<String>) -> Self {
        Self::styled(value, TextRole::Caption)
    }

    pub fn metadata(value: impl Into<String>) -> Self {
        Self::caption(value).tone(TextTone::Tertiary)
    }

    pub fn style(mut self, role: TextRole) -> Self {
        self.role = role;
        self
    }

    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        self.font_family = Some(font_family.into());

        self
    }

    pub fn monospaced(mut self) -> Self {
        self.font_family = Some(DEFAULT_MONOSPACE_FONT_FAMILY.to_owned());
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(finite_positive_or(font_size, Typography::DEFAULT.body.size));

        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(finite_positive_or(
            line_height,
            self.font_size
                .unwrap_or(Typography::DEFAULT.body.line_height),
        ));

        self
    }

    pub fn weight(mut self, weight: u16) -> Self {
        self.weight = Some(weight.clamp(1, 1000));

        self
    }

    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;

        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn cache_layout(mut self, cache_layout: bool) -> Self {
        self.cache_layout = cache_layout;
        self
    }

    pub fn measure_text(&self, measurer: &mut TextMeasurer, maximum_width: Option<f32>) -> Size {
        self.measure_text_with_typography(measurer, &Typography::DEFAULT, maximum_width)
    }

    pub fn tone(mut self, tone: TextTone) -> Self {
        self.tone = tone;
        self
    }

    pub(crate) fn measure_text_with_typography(
        &self,
        measurer: &mut TextMeasurer,
        typography: &Typography,
        maximum_width: Option<f32>,
    ) -> Size {
        if self.value.is_empty() {
            return Size::new(0.0, 0.0);
        }

        let style = self.resolved_style(typography);
        let font_scale = measurer.font_scale();
        let font_size = resolved_font_size(style.size) * font_scale;
        let line_height = resolved_line_height(font_size, style.line_height * font_scale);
        let metrics = Metrics::new(font_size, line_height);
        let font_system = measurer.font_system_mut();
        let mut buffer = Buffer::new(font_system, metrics);
        let attrs = self.create_attrs(typography);
        let maximum_width = normalize_maximum_width(maximum_width);
        let mut buffer = buffer.borrow_with(font_system);

        /*
         * 高さをNoneにすることで、
         * 全行を計測対象にします。
         */
        buffer.set_size(maximum_width, None);
        buffer.set_text(
            self.value.as_str(),
            &attrs,
            Shaping::Advanced,
            self.alignment.to_cosmic(),
        );

        let mut measured_width = 0.0_f32;
        let mut measured_height = 0.0_f32;

        for run in buffer.layout_runs() {
            measured_width = measured_width.max(run.line_w);
            measured_height = measured_height.max(run.line_top + run.line_height);
        }

        if measured_width <= 0.0 || measured_height <= 0.0 {
            return self.measure_text_without_font(typography, maximum_width, font_scale);
        }

        if let Some(maximum_width) = maximum_width {
            measured_width = measured_width.min(maximum_width);
        }

        Size::new(
            measured_width.max(0.0).ceil(),
            measured_height.max(0.0).ceil(),
        )
    }

    pub fn measure_unbounded(&self, measurer: &mut TextMeasurer) -> Size {
        self.measure_text(measurer, None)
    }

    pub(crate) fn measure_unbounded_with_typography(
        &self,
        measurer: &mut TextMeasurer,
        typography: &Typography,
    ) -> Size {
        self.measure_text_with_typography(measurer, typography, None)
    }

    fn create_attrs(&self, typography: &Typography) -> Attrs<'_> {
        let style = self.resolved_style(typography);
        Attrs::new()
            .family(resolve_font_family(self.resolved_family(style.family)))
            .weight(Weight(style.weight.0.clamp(1, 1000)))
    }

    fn measure_text_without_font(
        &self,
        typography: &Typography,
        maximum_width: Option<f32>,
        font_scale: f32,
    ) -> Size {
        if self.value.is_empty() {
            return Size::new(0.0, 0.0);
        }

        let style = self.resolved_style(typography);
        let font_size = resolved_font_size(style.size) * font_scale;
        let line_height = resolved_line_height(font_size, style.line_height * font_scale);
        let glyph_width = (font_size * 0.56).max(1.0);
        let max_width = normalize_maximum_width(maximum_width);
        let mut line_count = 0usize;
        let mut measured_width = 0.0_f32;

        for line in self.value.split('\n') {
            let line_width = line.chars().count() as f32 * glyph_width;
            if let Some(max_width) = max_width.filter(|width| *width > 0.0) {
                let wrapped_lines = (line_width / max_width).ceil().max(1.0);
                line_count = line_count.saturating_add(wrapped_lines as usize);
                measured_width = measured_width.max(line_width.min(max_width));
            } else {
                line_count = line_count.saturating_add(1);
                measured_width = measured_width.max(line_width);
            }
        }

        Size::new(
            measured_width.max(0.0).ceil(),
            (line_count.max(1) as f32 * line_height).ceil(),
        )
    }

    fn resolved_style(&self, typography: &Typography) -> crate::typography::TextStyle {
        let mut style = typography.style(self.role);
        if let Some(size) = self.font_size {
            style.size = size;
        }
        if let Some(line_height) = self.line_height {
            style.line_height = line_height;
        }
        if let Some(weight) = self.weight {
            style.weight = FontWeight(weight);
        }
        style
    }

    fn resolved_family(&self, family: FontFamily) -> &str {
        self.font_family.as_deref().unwrap_or(match family {
            FontFamily::Sans => DEFAULT_UI_FONT_FAMILY,
            FontFamily::Monospace => DEFAULT_MONOSPACE_FONT_FAMILY,
        })
    }
}

impl IntoViewNode for Text {
    fn into_view_node(self, context: &mut ViewNodeContext) -> ViewNode {
        let typography = Typography::DEFAULT;
        let style = self.resolved_style(&typography);
        let font_family = self.resolved_family(style.family).to_owned();

        ViewNode::new(
            context.allocate_node_id(),
            ViewNodeKind::Text(TextNode {
                content: self.value,
                font_family,
                font_size: style.size,
                line_height: style.line_height,
                weight: style.weight.0,
                alignment: self.alignment,
                color: self
                    .color
                    .unwrap_or_else(|| self.tone.resolve(&Theme::current())),
                cache_layout: self.cache_layout,
            }),
        )
    }
}

impl View for Text {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let maximum_width = if constraints.maximum.width.is_finite() {
            Some(constraints.maximum.width.max(0.0))
        } else {
            None
        };

        let measured = self.measure_text_with_typography(
            context.text_measurer,
            context.typography,
            maximum_width,
        );

        constraints.constrain(measured)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 || self.value.is_empty() {
            return;
        }

        let style = self.resolved_style(context.typography);
        let font_scale = context.text_measurer.font_scale();
        let font_size = resolved_font_size(style.size) * font_scale;

        let line_height = resolved_line_height(font_size, style.line_height * font_scale);

        context.display_list.push(DrawCommand::DrawText {
            command: TextCommand {
                text: self.value.clone(),

                bounds,
                cache_layout: self.cache_layout,

                font_family: self.resolved_family(style.family).to_owned(),

                font_size,

                line_height,

                weight: style.weight.0.clamp(1, 1000),

                alignment: self.alignment,

                color: self
                    .color
                    .unwrap_or_else(|| self.tone.resolve(context.theme)),
            },
        });
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextTone {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Disabled,
    Accent,
    Destructive,
}

impl TextTone {
    fn resolve(self, theme: &Theme) -> Color {
        match self {
            Self::Primary => theme.colors.text_primary,
            Self::Secondary => theme.colors.text_secondary,
            Self::Tertiary => theme.colors.text_tertiary,
            Self::Disabled => theme.colors.text_disabled,
            Self::Accent => theme.colors.accent,
            Self::Destructive => theme.colors.destructive,
        }
    }
}

fn normalize_maximum_width(maximum_width: Option<f32>) -> Option<f32> {
    maximum_width.map(|width| {
        if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        }
    })
}

fn resolved_font_size(font_size: f32) -> f32 {
    finite_positive_or(font_size, 16.0)
}

fn resolved_line_height(font_size: f32, line_height: f32) -> f32 {
    finite_positive_or(line_height, font_size).max(font_size)
}

fn finite_positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::Text;
    use crate::font::DEFAULT_MONOSPACE_FONT_FAMILY;

    #[test]
    fn monospaced_uses_the_embedded_monospace_family() {
        let text = Text::new("terminal").monospaced();

        assert_eq!(
            text.font_family.as_deref(),
            Some(DEFAULT_MONOSPACE_FONT_FAMILY)
        );
    }

    #[test]
    fn dynamic_text_can_disable_layout_caching() {
        let text = Text::new("changing").cache_layout(false);

        assert!(!text.cache_layout);
    }
}
