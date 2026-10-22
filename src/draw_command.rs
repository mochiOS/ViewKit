//! レンダラーへ渡す描画命令を定義

use crate::geometry::{Point, Rect};
use crate::image::ImageData;
use crate::svg::SvgData;
use crate::theme::Color;
use crate::typography::TextAlignment;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub enum DrawCommand {
    /// A closed native contour. Points may include concave sections.
    DrawContour {
        points: Vec<Point>,
        fill: Color,
        stroke: Color,
        stroke_width: f32,
    },
    Clear {
        color: Color,
    },

    FillRect {
        rect: Rect,
        color: Color,
    },

    FillRoundedRect {
        rect: Rect,
        radius: f32,
        color: Color,
    },

    FillEllipse {
        rect: Rect,
        color: Color,
    },

    StrokeEllipse {
        rect: Rect,
        color: Color,
        width: f32,
    },

    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
    },

    StrokeRoundedRect {
        rect: Rect,
        radius: f32,
        color: Color,
        width: f32,
    },

    DrawText {
        command: TextCommand,
    },

    DrawImage {
        command: ImageCommand,
    },

    DrawSvg {
        command: SvgCommand,
    },

    PushClip {
        rect: Rect,
    },

    PushRoundedClip {
        rect: Rect,
        radius: f32,
    },

    PopClip,
    /// A border entirely inside the continuous boundary; radius is unchanged.
    StrokeInsideRoundedRect {
        rect: Rect,
        radius: f32,
        color: Color,
        width: f32,
    },
    /// Independent radii ordered top-left, top-right, bottom-right, bottom-left.
    FillRoundedCornersRect {
        rect: Rect,
        radii: [f32; 4],
        color: Color,
    },
    StrokeInsideRoundedCornersRect {
        rect: Rect,
        radii: [f32; 4],
        color: Color,
        width: f32,
    },
    PushRoundedCornersClip {
        rect: Rect,
        radii: [f32; 4],
    },
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct DisplayList {
    commands: Vec<DrawCommand>,
}

/// レイアウトを変更せず、描画命令へ適用する一様拡縮と移動です。
/// opacityは各描画要素に掛かります（オフスクリーン合成のgroup opacityではありません）。
#[derive(Clone, Copy, Debug)]
pub struct PaintTransform {
    pub anchor: Point,
    pub scale: f32,
    pub offset: Point,
    pub opacity: f32,
}

impl PaintTransform {
    pub fn point(self, point: Point) -> Point {
        Point::new(
            self.anchor.x + (point.x - self.anchor.x) * self.scale + self.offset.x,
            self.anchor.y + (point.y - self.anchor.y) * self.scale + self.offset.y,
        )
    }

    pub fn inverse_point(self, point: Point) -> Point {
        Point::new(
            self.anchor.x + (point.x - self.anchor.x - self.offset.x) / self.scale,
            self.anchor.y + (point.y - self.anchor.y - self.offset.y) / self.scale,
        )
    }

    pub fn rect(self, rect: Rect) -> Rect {
        let origin = self.point(rect.origin);
        Rect::new(
            origin.x,
            origin.y,
            rect.size.width * self.scale,
            rect.size.height * self.scale,
        )
    }

    fn command(self, command: &mut DrawCommand) {
        match command {
            DrawCommand::DrawContour {
                points,
                fill,
                stroke,
                stroke_width,
            } => {
                for point in points {
                    *point = self.point(*point);
                }
                *fill = fill.alpha(self.opacity);
                *stroke = stroke.alpha(self.opacity);
                *stroke_width *= self.scale;
            }
            DrawCommand::Clear { .. } | DrawCommand::PopClip => {}
            DrawCommand::FillRect { rect, color } | DrawCommand::FillEllipse { rect, color } => {
                *rect = self.rect(*rect);
                *color = color.alpha(self.opacity);
            }
            DrawCommand::FillRoundedRect {
                rect,
                radius,
                color,
            } => {
                *rect = self.rect(*rect);
                *radius *= self.scale;
                *color = color.alpha(self.opacity);
            }
            DrawCommand::StrokeRect { rect, color, width }
            | DrawCommand::StrokeEllipse { rect, color, width } => {
                *rect = self.rect(*rect);
                *width *= self.scale;
                *color = color.alpha(self.opacity);
            }
            DrawCommand::StrokeInsideRoundedRect {
                rect,
                radius,
                color,
                width,
            }
            | DrawCommand::StrokeRoundedRect {
                rect,
                radius,
                color,
                width,
            } => {
                *rect = self.rect(*rect);
                *radius *= self.scale;
                *width *= self.scale;
                *color = color.alpha(self.opacity);
            }
            DrawCommand::FillRoundedCornersRect { rect, radii, color } => {
                *rect = self.rect(*rect);
                *radii = radii.map(|r| r * self.scale);
                *color = color.alpha(self.opacity);
            }
            DrawCommand::StrokeInsideRoundedCornersRect {
                rect,
                radii,
                color,
                width,
            } => {
                *rect = self.rect(*rect);
                *radii = radii.map(|r| r * self.scale);
                *color = color.alpha(self.opacity);
                *width *= self.scale;
            }
            DrawCommand::PushRoundedCornersClip { rect, radii } => {
                *rect = self.rect(*rect);
                *radii = radii.map(|r| r * self.scale);
            }
            DrawCommand::DrawText { command } => {
                command.bounds = self.rect(command.bounds);
                command.font_size *= self.scale;
                command.line_height *= self.scale;
                command.color = command.color.alpha(self.opacity);
            }
            DrawCommand::DrawImage { command } => {
                command.bounds = self.rect(command.bounds);
                command.opacity *= self.opacity;
            }
            DrawCommand::DrawSvg { command } => {
                command.bounds = self.rect(command.bounds);
                command.opacity *= self.opacity;
            }
            DrawCommand::PushClip { rect } => *rect = self.rect(*rect),
            DrawCommand::PushRoundedClip { rect, radius } => {
                *rect = self.rect(*rect);
                *radius *= self.scale;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct TextCommand {
    pub text: String,
    pub bounds: Rect,
    pub cache_layout: bool,

    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: u16,
    pub alignment: TextAlignment,

    pub color: Color,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub enum ImageSampling {
    Nearest,

    Bilinear,

    #[default]
    Bicubic,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageCommand {
    pub image: ImageData,
    pub bounds: Rect,

    pub opacity: f32,
    pub sampling: ImageSampling,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct SvgCommand {
    pub svg: SvgData,
    pub bounds: Rect,
    pub opacity: f32,
    // 単色シンボル用
    pub tint: Option<Color>,
}

impl DisplayList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub(crate) fn transform_since(&mut self, start: usize, transform: PaintTransform) {
        assert!(transform.scale.is_finite() && transform.scale > 0.0);
        for command in &mut self.commands[start..] {
            transform.command(command);
        }
    }
}

pub fn clamp_radius(rect: Rect, radius: f32) -> f32 {
    crate::continuous_corner::shape(rect, [radius; 4]).radii[0]
}

#[cfg(test)]
mod presentation_tests {
    use super::*;

    #[test]
    fn presentation_preserves_layout_ratios_clips_and_preceding_commands() {
        let transform = PaintTransform {
            anchor: Point::new(10.0, 20.0),
            scale: 0.5,
            offset: Point::new(2.0, -3.0),
            opacity: 0.5,
        };
        let bounds = Rect::new(30.0, 40.0, 100.0, 24.0);
        assert_eq!(
            transform.inverse_point(transform.point(bounds.origin)),
            bounds.origin
        );
        let mut list = DisplayList::new();
        list.push(DrawCommand::Clear {
            color: Color::BLACK,
        });
        list.push(DrawCommand::PushRoundedClip {
            rect: bounds,
            radius: 8.0,
        });
        list.push(DrawCommand::DrawText {
            command: TextCommand {
                text: "menu".into(),
                bounds,
                cache_layout: true,
                font_family: "sans-serif".into(),
                font_size: 14.0,
                line_height: 20.0,
                weight: 400,
                alignment: TextAlignment::Start,
                color: Color::WHITE,
            },
        });
        list.push(DrawCommand::PopClip);
        list.transform_since(1, transform);
        assert_eq!(
            list.commands()[0],
            DrawCommand::Clear {
                color: Color::BLACK
            }
        );
        assert_eq!(
            list.commands()[1],
            DrawCommand::PushRoundedClip {
                rect: Rect::new(22.0, 27.0, 50.0, 12.0),
                radius: 4.0
            }
        );
        let DrawCommand::DrawText { command } = &list.commands()[2] else {
            panic!()
        };
        assert_eq!(
            command.bounds.size.width / command.font_size,
            bounds.size.width / 14.0
        );
        assert_eq!(command.font_size, 7.0);
        assert_eq!(command.line_height, 10.0);
        assert_eq!(command.color, Color::WHITE.alpha(0.5));
        assert_eq!(list.commands()[3], DrawCommand::PopClip);
    }
}
