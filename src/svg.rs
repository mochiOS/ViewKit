//! SVGリソースを管理する型を定義

use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use resvg::usvg;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SvgError {
    #[error("SVGファイルを読み込めませんでした: {0}")]
    Io(#[from] std::io::Error),

    #[error("SVGを解析できませんでした: {0}")]
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

    pub(crate) fn tree(&self) -> &usvg::Tree {
        &self.inner.tree
    }

    fn decode_with_options(bytes: &[u8], options: &usvg::Options<'_>) -> Result<Self, SvgError> {
        let tree = usvg::Tree::from_data(bytes, options)?;

        let size = tree.size();

        Ok(Self {
            inner: Arc::new(SvgDataInner {
                tree,

                width: size.width(),
                height: size.height(),
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

impl Hash for SvgData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.inner).hash(state);
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
