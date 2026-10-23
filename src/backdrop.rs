//! Sparse background sampling for overlay contrast, without GPU readback.
use crate::{
    draw_command::{DisplayList, DrawCommand},
    geometry::{Point, Rect},
    theme::Color,
};

impl DisplayList {
    /// Samples underlying fills and images at five positions for overlay contrast.
    /// Text, symbols and strokes are excluded so foreground details do not cause flicker.
    pub fn sample_background(&self, bounds: Rect, fallback: Color) -> Color {
        let mut sum = [0u32; 3];
        for (x, y) in [(0.2, 0.3), (0.5, 0.3), (0.8, 0.3), (0.3, 0.7), (0.7, 0.7)] {
            let point = Point::new(
                bounds.origin.x + bounds.size.width * x,
                bounds.origin.y + bounds.size.height * y,
            );
            let color = self.background_at(point, fallback);
            sum[0] += u32::from(color.red);
            sum[1] += u32::from(color.green);
            sum[2] += u32::from(color.blue);
        }
        Color::rgb((sum[0] / 5) as u8, (sum[1] / 5) as u8, (sum[2] / 5) as u8)
    }
    fn background_at(&self, point: Point, fallback: Color) -> Color {
        let mut result = fallback;
        let mut clips: Vec<bool> = Vec::new();
        for command in self.commands() {
            let source = match command {
                DrawCommand::PushClip { rect } => {
                    clips.push(rect.contains(point));
                    continue;
                }
                DrawCommand::PushRoundedClip { rect, radius } => {
                    clips.push(rect.contains_rounded(*radius, point));
                    continue;
                }
                DrawCommand::PushRoundedCornersClip { rect, radii } => {
                    clips.push(
                        crate::continuous_corner::shape(*rect, *radii).contains((point.x, point.y)),
                    );
                    continue;
                }
                DrawCommand::PopClip => {
                    clips.pop();
                    continue;
                }
                DrawCommand::Clear { color } => {
                    result = *color;
                    continue;
                }
                _ if clips.iter().any(|inside| !inside) => continue,
                DrawCommand::FillRect { rect, color } if rect.contains(point) => Some(*color),
                DrawCommand::FillRoundedRect {
                    rect,
                    radius,
                    color,
                } if rect.contains_rounded(*radius, point) => Some(*color),
                DrawCommand::FillRoundedCornersRect { rect, radii, color }
                    if crate::continuous_corner::shape(*rect, *radii)
                        .contains((point.x, point.y)) =>
                {
                    Some(*color)
                }
                DrawCommand::DrawImage { command } if command.bounds.contains(point) => {
                    let rect = command.bounds;
                    let width = command.image.width() as usize;
                    let height = command.image.height() as usize;
                    let x = (((point.x - rect.origin.x) / rect.size.width * width as f32) as usize)
                        .min(width - 1);
                    let y = (((point.y - rect.origin.y) / rect.size.height * height as f32)
                        as usize)
                        .min(height - 1);
                    let pixels = command.image.premultiplied_rgba8();
                    let pixel = &pixels[(y * width + x) * 4..][..4];
                    let alpha = u32::from(pixel[3]);
                    let channel = |index: usize| {
                        if alpha == 0 {
                            0
                        } else {
                            ((u32::from(pixel[index]) * 255 + alpha / 2) / alpha).min(255) as u8
                        }
                    };
                    Some(Color::rgba(
                        channel(0),
                        channel(1),
                        channel(2),
                        (alpha as f32 * command.opacity.clamp(0.0, 1.0)).round() as u8,
                    ))
                }
                _ => None,
            };
            if let Some(source) = source {
                let alpha = u32::from(source.alpha);
                let over = |s: u8, d: u8| {
                    ((u32::from(s) * alpha + u32::from(d) * (255 - alpha) + 127) / 255) as u8
                };
                result = Color::rgb(
                    over(source.red, result.red),
                    over(source.green, result.green),
                    over(source.blue, result.blue),
                );
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        draw_command::{ImageCommand, ImageSampling},
        image::ImageData,
    };
    #[test]
    fn samples_actual_images_and_respects_window_clips_and_transparency() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut list = DisplayList::new();
        list.push(DrawCommand::DrawImage {
            command: ImageCommand {
                image: ImageData::from_rgba8(1, 1, vec![20, 30, 40, 255]).unwrap(),
                bounds: rect,
                opacity: 1.0,
                sampling: ImageSampling::Nearest,
            },
        });
        assert_eq!(
            list.sample_background(rect, Color::WHITE),
            Color::rgb(20, 30, 40)
        );
        list.push(DrawCommand::PushClip {
            rect: Rect::new(200.0, 200.0, 10.0, 10.0),
        });
        list.push(DrawCommand::FillRect {
            rect,
            color: Color::WHITE,
        });
        list.push(DrawCommand::PopClip);
        assert_eq!(
            list.sample_background(rect, Color::WHITE),
            Color::rgb(20, 30, 40)
        );
        list.push(DrawCommand::FillRoundedRect {
            rect,
            radius: 10.0,
            color: Color::rgba(255, 255, 255, 128),
        });
        assert_eq!(
            list.sample_background(rect, Color::BLACK),
            Color::rgb(138, 143, 148)
        );
    }
}
