//! ViewKitで使用する画像データ

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::svg::SvgData;

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("Image dimensions must be greater than zero: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Image dimensions are too large: {width}x{height}")]
    SizeOverflow { width: u32, height: u32 },

    #[error("Invalid RGBA image data length: expected={expected}, actual={actual}")]
    InvalidPixelLength { expected: usize, actual: usize },

    #[error("Failed to decode the image: {0}")]
    Decode(#[from] image::ImageError),
}

#[derive(Clone)]
pub struct ImageData {
    inner: Arc<ImageDataInner>,
}

struct ImageDataInner {
    width: u32,
    height: u32,
    pixels: Box<[u8]>,
}

impl ImageData {
    pub fn from_rgba8(
        width: u32,
        height: u32,
        pixels: impl Into<Vec<u8>>,
    ) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions { width, height });
        }

        let expected = expected_pixel_length(width, height)
            .ok_or(ImageError::SizeOverflow { width, height })?;

        let mut pixels = pixels.into();

        if pixels.len() != expected {
            return Err(ImageError::InvalidPixelLength {
                expected,
                actual: pixels.len(),
            });
        }

        premultiply_rgba8(&mut pixels);

        Ok(Self {
            inner: Arc::new(ImageDataInner {
                width,
                height,
                pixels: pixels.into_boxed_slice(),
            }),
        })
    }

    pub fn from_premultiplied_rgba8(
        width: u32,
        height: u32,
        pixels: impl Into<Vec<u8>>,
    ) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions { width, height });
        }

        let expected = expected_pixel_length(width, height)
            .ok_or(ImageError::SizeOverflow { width, height })?;

        let pixels = pixels.into();

        if pixels.len() != expected {
            return Err(ImageError::InvalidPixelLength {
                expected,
                actual: pixels.len(),
            });
        }

        Ok(Self {
            inner: Arc::new(ImageDataInner {
                width,
                height,
                pixels: pixels.into_boxed_slice(),
            }),
        })
    }

    pub fn from_svg(svg: &SvgData, width: u32, height: u32) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions { width, height });
        }
        let svg_width = svg.width();
        let svg_height = svg.height();
        if !svg_width.is_finite()
            || !svg_height.is_finite()
            || svg_width <= 0.0
            || svg_height <= 0.0
        {
            return Err(ImageError::InvalidDimensions { width, height });
        }
        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .ok_or(ImageError::SizeOverflow { width, height })?;
        let transform =
            tiny_skia::Transform::from_scale(width as f32 / svg_width, height as f32 / svg_height);
        resvg::render(svg.tree(), transform, &mut pixmap.as_mut());
        Self::from_premultiplied_rgba8(width, height, pixmap.data().to_vec())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ImageError> {
        let image = ::image::load_from_memory(bytes)?.into_rgba8();

        let width = image.width();
        let height = image.height();

        Self::from_rgba8(width, height, image.into_raw())
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let bytes = std::fs::read(path).map_err(::image::ImageError::IoError)?;
        Self::decode(&bytes)
    }

    pub fn thumbnail_from_path(
        path: impl AsRef<Path>,
        max_width: u32,
        max_height: u32,
    ) -> Result<Self, ImageError> {
        if max_width == 0 || max_height == 0 {
            return Err(ImageError::InvalidDimensions {
                width: max_width,
                height: max_height,
            });
        }

        let image = ::image::open(path)?;
        let image = image.thumbnail(max_width, max_height).into_rgba8();
        let width = image.width();
        let height = image.height();
        Self::from_rgba8(width, height, image.into_raw())
    }

    pub fn width(&self) -> u32 {
        self.inner.width
    }

    pub fn height(&self) -> u32 {
        self.inner.height
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.inner.width, self.inner.height)
    }

    /// Returns a lossless clockwise quarter-turn without re-encoding the image.
    pub fn rotated_quarter_turns(&self, turns: u8) -> Result<Self, ImageError> {
        let turns = turns % 4;
        if turns == 0 {
            return Ok(self.clone());
        }
        let (width, height) = self.dimensions();
        let (out_width, out_height) = if turns % 2 == 0 {
            (width, height)
        } else {
            (height, width)
        };
        let mut pixels = vec![0; self.inner.pixels.len()];
        for y in 0..height {
            for x in 0..width {
                let (dx, dy) = match turns {
                    1 => (height - 1 - y, x),
                    2 => (width - 1 - x, height - 1 - y),
                    _ => (y, width - 1 - x),
                };
                let source = (y as usize * width as usize + x as usize) * 4;
                let destination = (dy as usize * out_width as usize + dx as usize) * 4;
                pixels[destination..destination + 4]
                    .copy_from_slice(&self.inner.pixels[source..source + 4]);
            }
        }
        Self::from_premultiplied_rgba8(out_width, out_height, pixels)
    }

    pub(crate) fn premultiplied_rgba8(&self) -> &[u8] {
        &self.inner.pixels
    }
}

impl PartialEq for ImageData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ImageData {}

#[cfg(target_os = "linux")]
impl serde::Serialize for ImageData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(
            &(self.width(), self.height(), self.premultiplied_rgba8()),
            serializer,
        )
    }
}

#[cfg(target_os = "linux")]
impl<'de> serde::Deserialize<'de> for ImageData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (width, height, pixels): (u32, u32, Vec<u8>) =
            serde::Deserialize::deserialize(deserializer)?;
        Self::from_premultiplied_rgba8(width, height, pixels).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for ImageData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageData")
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

fn expected_pixel_length(width: u32, height: u32) -> Option<usize> {
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;

    width.checked_mul(height)?.checked_mul(4)
}

fn premultiply_rgba8(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[3];

        pixel[0] = premultiply_channel(pixel[0], alpha);

        pixel[1] = premultiply_channel(pixel[1], alpha);

        pixel[2] = premultiply_channel(pixel[2], alpha);
    }
}

fn premultiply_channel(channel: u8, alpha: u8) -> u8 {
    let channel = u16::from(channel);
    let alpha = u16::from(alpha);

    ((channel * alpha + 127) / 255) as u8
}

#[cfg(test)]
mod rotation_tests {
    use super::*;
    #[test]
    fn rectangular_quarter_turns_preserve_pixel_order_and_premultiplied_alpha() {
        let image = ImageData::from_premultiplied_rgba8(
            2,
            3,
            (1..=6).flat_map(|v| [v, 0, 0, 64]).collect::<Vec<_>>(),
        )
        .unwrap();
        for (turns, dimensions, expected) in [
            (1, (3, 2), vec![5, 3, 1, 6, 4, 2]),
            (2, (2, 3), vec![6, 5, 4, 3, 2, 1]),
            (3, (3, 2), vec![2, 4, 6, 1, 3, 5]),
        ] {
            let result = image.rotated_quarter_turns(turns).unwrap();
            assert_eq!(result.dimensions(), dimensions);
            assert_eq!(
                result
                    .premultiplied_rgba8()
                    .chunks_exact(4)
                    .map(|p| p[0])
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(
                result
                    .premultiplied_rgba8()
                    .chunks_exact(4)
                    .all(|p| p[3] == 64)
            );
        }
        assert_eq!(image.rotated_quarter_turns(4).unwrap(), image);
    }
}
