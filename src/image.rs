//! ViewKitで使用する画像データ

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::svg::SvgData;

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("画像サイズは0より大きくなければなりません: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("画像サイズが大きすぎます: {width}x{height}")]
    SizeOverflow { width: u32, height: u32 },

    #[error("RGBA画像データの長さが不正です: expected={expected}, actual={actual}")]
    InvalidPixelLength { expected: usize, actual: usize },

    #[error("画像のデコードに失敗しました: {0}")]
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
        let image = ::image::open(path)?.into_rgba8();

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
