//! Shared, full-color system artwork, separate from tintable VK Symbols.

use crate::svg::SvgData;
use std::sync::OnceLock;

/// Built-in system artwork rendered with its original colors using [`crate::components::Svg`].
///
/// These assets are embedded in ViewKit; applications do not need a Files.app
/// installation, resource path, or filesystem access to display them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemIcon {
    /// The blue folder artwork used by Files.app, including its original canvas padding.
    Folder,
    /// The generic document artwork used when no application supplies a file icon.
    File,
}

impl SystemIcon {
    /// Returns the shared vector artwork, decoding each built-in asset only once.
    ///
    /// Clones share the parsed SVG. Use `Svg::new(SystemIcon::Folder.svg())`
    /// with a layout frame to select the display size without rasterizing it.
    /// VK Symbols and `Icon::new()` continue to provide tintable interface glyphs.
    pub fn svg(self) -> SvgData {
        match self {
            Self::Folder => {
                static FOLDER: OnceLock<SvgData> = OnceLock::new();
                FOLDER
                    .get_or_init(|| {
                        SvgData::decode(include_bytes!("../resources/system-icons/folder.svg"))
                            .expect("the built-in folder SVG must be valid")
                    })
                    .clone()
            }
            Self::File => {
                static FILE: OnceLock<SvgData> = OnceLock::new();
                FILE.get_or_init(|| {
                    SvgData::decode(include_bytes!("../resources/system-icons/file.svg"))
                        .expect("the built-in file SVG must be valid")
                })
                .clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_artwork_is_shared_and_retains_its_original_canvas() {
        let first = SystemIcon::Folder.svg();
        assert_eq!(first, SystemIcon::Folder.svg());
        assert_eq!(first.size(), (64.0, 64.0));
    }

    #[test]
    fn folder_artwork_keeps_blue_color_and_transparent_padding() {
        let folder = SystemIcon::Folder.svg();
        let mut pixels = tiny_skia::Pixmap::new(64, 64).unwrap();
        resvg::render(
            folder.tree(),
            tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        assert_eq!(pixels.pixel(0, 0).unwrap().alpha(), 0);
        let center = pixels.pixel(32, 32).unwrap();
        assert_eq!(center.alpha(), 255);
        assert!(center.blue() > center.green() && center.green() > center.red());
    }

    #[test]
    fn file_artwork_is_shared_and_preserves_its_document_face() {
        let file = SystemIcon::File.svg();
        assert_eq!(file, SystemIcon::File.svg());
        assert_eq!(file.size(), (64.0, 64.0));
        let mut pixels = tiny_skia::Pixmap::new(64, 64).unwrap();
        resvg::render(
            file.tree(),
            tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        assert_eq!(pixels.pixel(0, 0).unwrap().alpha(), 0);
        let face = pixels.pixel(32, 32).unwrap();
        assert_eq!(face.alpha(), 255);
        assert!(face.red() > 235 && face.green() > 235 && face.blue() > 235);
    }
}
