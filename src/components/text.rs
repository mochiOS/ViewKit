//! 文字列を描画するTextを定義

use cosmic_text::{
    Attrs,
    Buffer,
    Family,
    Metrics,
    Shaping,
    Weight,
};

use crate::draw_command::{
    DrawCommand,
    TextCommand,
};
use crate::geometry::{
    Rect,
    Size,
};
use crate::theme::Color;
use crate::typography::{
    TextAlignment,
    TextMeasurer,
};
use crate::view::{
    PaintContext,
    View,
};

pub struct Text {
    value: String,

    font_family: String,
    font_size: f32,
    line_height: f32,
    weight: u16,

    alignment:
        TextAlignment,

    color: Color,
}

impl Text {
    pub fn new(
        value: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),

            font_family:
            String::from(
                "Noto Sans JP",
            ),

            font_size: 16.0,
            line_height: 24.0,
            weight: 400,

            alignment:
            TextAlignment::Start,

            color: Color::BLACK,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn font_family(
        mut self,
        font_family: impl Into<String>,
    ) -> Self {
        self.font_family =
            font_family.into();

        self
    }

    pub fn font_size(
        mut self,
        font_size: f32,
    ) -> Self {
        self.font_size =
            finite_positive_or(
                font_size,
                16.0,
            );

        self
    }

    pub fn line_height(
        mut self,
        line_height: f32,
    ) -> Self {
        self.line_height =
            finite_positive_or(
                line_height,
                self.font_size,
            );

        self
    }

    pub fn weight(
        mut self,
        weight: u16,
    ) -> Self {
        self.weight =
            weight.clamp(
                1,
                1000,
            );

        self
    }

    pub fn alignment(
        mut self,
        alignment:
        TextAlignment,
    ) -> Self {
        self.alignment =
            alignment;

        self
    }

    pub fn color(
        mut self,
        color: Color,
    ) -> Self {
        self.color = color;
        self
    }

    pub fn measure(
        &self,
        measurer:
        &mut TextMeasurer,

        maximum_width:
        Option<f32>,
    ) -> Size {
        if self.value.is_empty() {
            return Size::new(
                0.0,
                0.0,
            );
        }

        let font_size =
            resolved_font_size(
                self.font_size,
            );

        let line_height =
            resolved_line_height(
                font_size,
                self.line_height,
            );

        let metrics =
            Metrics::new(
                font_size,
                line_height,
            );

        let font_system =
            measurer
                .font_system_mut();

        let mut buffer =
            Buffer::new(
                font_system,
                metrics,
            );

        let attrs =
            self.create_attrs();

        let maximum_width =
            normalize_maximum_width(
                maximum_width,
            );

        let mut buffer =
            buffer.borrow_with(
                font_system,
            );

        /*
         * 高さをNoneにすることで、
         * 全行を計測対象にします。
         */
        buffer.set_size(
            maximum_width,
            None,
        );

        buffer.set_text(
            self.value.as_str(),
            &attrs,
            Shaping::Advanced,
            self.alignment
                .to_cosmic(),
        );

        let mut measured_width =
            0.0_f32;

        let mut measured_height =
            0.0_f32;

        for run in buffer.layout_runs() {
            measured_width =
                measured_width.max(
                    run.line_w,
                );

            measured_height =
                measured_height.max(
                    run.line_top
                        + run.line_height,
                );
        }

        if let Some(maximum_width) =
            maximum_width
        {
            measured_width =
                measured_width.min(
                    maximum_width,
                );
        }

        Size::new(
            measured_width
                .max(0.0)
                .ceil(),

            measured_height
                .max(0.0)
                .ceil(),
        )
    }

    pub fn measure_unbounded(
        &self,
        measurer:
        &mut TextMeasurer,
    ) -> Size {
        self.measure(
            measurer,
            None,
        )
    }

    fn create_attrs(
        &self,
    ) -> Attrs<'_> {
        Attrs::new()
            .family(
                Family::Name(
                    self.font_family
                        .as_str(),
                ),
            )
            .weight(
                Weight(
                    self.weight.clamp(
                        1,
                        1000,
                    ),
                ),
            )
    }
}

impl View for Text {
    fn paint(
        &self,
        bounds: Rect,
        context:
        &mut PaintContext<'_>,
    ) {
        if bounds.size.width <= 0.0
            || bounds.size.height <= 0.0
            || self.value.is_empty()
        {
            return;
        }

        let font_size =
            resolved_font_size(
                self.font_size,
            );

        let line_height =
            resolved_line_height(
                font_size,
                self.line_height,
            );

        context.display_list.push(
            DrawCommand::PushClip {
                rect: bounds,
            },
        );

        context.display_list.push(
            DrawCommand::DrawText {
                command: TextCommand {
                    text:
                    self.value.clone(),

                    bounds,

                    font_family:
                    self.font_family
                        .clone(),

                    font_size,

                    line_height,

                    weight:
                    self.weight.clamp(
                        1,
                        1000,
                    ),

                    alignment:
                    self.alignment,

                    color:
                    self.color,
                },
            },
        );

        context.display_list.push(
            DrawCommand::PopClip,
        );
    }
}

fn normalize_maximum_width(
    maximum_width: Option<f32>,
) -> Option<f32> {
    maximum_width.map(
        |width| {
            if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            }
        },
    )
}

fn resolved_font_size(
    font_size: f32,
) -> f32 {
    finite_positive_or(
        font_size,
        16.0,
    )
}

fn resolved_line_height(
    font_size: f32,
    line_height: f32,
) -> f32 {
    finite_positive_or(
        line_height,
        font_size,
    )
        .max(font_size)
}

fn finite_positive_or(
    value: f32,
    fallback: f32,
) -> f32 {
    if value.is_finite()
        && value > 0.0
    {
        value
    } else {
        fallback
    }
}