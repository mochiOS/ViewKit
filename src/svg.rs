//! SVGリソースを管理する型を定義

use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use resvg::usvg;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SvgError {
    #[error("Failed to read the SVG file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse the SVG: {0}")]
    Parse(#[from] usvg::Error),
}

#[derive(Clone)]
pub struct SvgData {
    inner: Arc<SvgDataInner>,
}

struct SvgDataInner {
    tree: usvg::Tree,

    width: f32,
    height: f32,
    content_bounds: SvgContentBounds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SvgContentBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SvgData {
    pub fn decode(bytes: &[u8]) -> Result<Self, SvgError> {
        Self::decode_with_options(bytes, &usvg::Options::default())
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SvgError> {
        let path = path.as_ref();

        let bytes = fs::read(path)?;

        let options = usvg::Options {
            resources_dir: path.parent().map(Path::to_path_buf),

            ..usvg::Options::default()
        };

        Self::decode_with_options(&bytes, &options)
    }

    pub fn width(&self) -> f32 {
        self.inner.width
    }

    pub fn height(&self) -> f32 {
        self.inner.height
    }

    pub fn size(&self) -> (f32, f32) {
        (self.inner.width, self.inner.height)
    }

    /// Rotates resolved vector artwork without reducing it to raster pixels.
    pub fn rotated_quarter_turns(&self, turns: u8) -> Result<Self, SvgError> {
        let turns = turns % 4;
        if turns == 0 {
            return Ok(self.clone());
        }
        let (width, height) = self.size();
        let (out_width, out_height) = if turns % 2 == 0 {
            (width, height)
        } else {
            (height, width)
        };
        let transform = match turns {
            1 => format!("translate({height} 0) rotate(90)"),
            2 => format!("translate({width} {height}) rotate(180)"),
            _ => format!("translate(0 {width}) rotate(270)"),
        };
        let artwork = self.tree().to_string(&usvg::WriteOptions::default());
        Self::decode(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{out_width}\" height=\"{out_height}\" viewBox=\"0 0 {out_width} {out_height}\"><g transform=\"{transform}\">{artwork}</g></svg>").as_bytes())
    }

    pub(crate) fn tree(&self) -> &usvg::Tree {
        &self.inner.tree
    }

    pub(crate) fn content_bounds(&self) -> SvgContentBounds {
        self.inner.content_bounds
    }

    fn decode_with_options(bytes: &[u8], options: &usvg::Options<'_>) -> Result<Self, SvgError> {
        let tree = usvg::Tree::from_data(bytes, options)?;

        let size = tree.size();
        let content = tree.root().abs_layer_bounding_box();

        Ok(Self {
            inner: Arc::new(SvgDataInner {
                tree,

                width: size.width(),
                height: size.height(),
                content_bounds: SvgContentBounds {
                    x: content.x(),
                    y: content.y(),
                    width: content.width(),
                    height: content.height(),
                },
            }),
        })
    }
}

impl PartialEq for SvgData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for SvgData {}

#[cfg(target_os = "linux")]
impl serde::Serialize for SvgData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.tree().to_string(&usvg::WriteOptions::default()))
    }
}

#[cfg(target_os = "linux")]
impl<'de> serde::Deserialize<'de> for SvgData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source: String = serde::Deserialize::deserialize(deserializer)?;
        Self::decode(source.as_bytes()).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for SvgData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SvgData")
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod rotation_tests {
    use super::*;
    #[test]
    fn vector_rotation_preserves_asymmetric_artwork_and_transparency() {
        let svg = SvgData::decode(br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="3"><rect x="0" y="0" width="1" height="1" fill="red"/></svg>"#).unwrap();
        let turned = svg.rotated_quarter_turns(1).unwrap();
        assert_eq!(turned.size(), (3.0, 2.0));
        let raster = crate::image::ImageData::from_svg(&turned, 3, 2).unwrap();
        let pixels = raster.premultiplied_rgba8();
        assert_eq!(&pixels[8..12], &[255, 0, 0, 255]);
        assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
        assert_eq!(svg.rotated_quarter_turns(4).unwrap(), svg);
    }
}
