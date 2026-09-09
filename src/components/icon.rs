//! Iconコンポーネント

use super::Svg;
use crate::geometry::Size;
use crate::svg::SvgData;
use crate::theme::Color;
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::sync::OnceLock;

const DEFAULT_ICON_SIZE: f32 = 24.0;

macro_rules! viewkit_svg {
    ($name:literal) => {{
        static DATA: OnceLock<SvgData> = OnceLock::new();

        DATA.get_or_init(|| {
            SvgData::decode(include_bytes!(concat!("../icons/", $name, ".svg"))).unwrap_or_else(
                |error| panic!("Failed to parse the `{}` icon SVG: {}", $name, error),
            )
        })
        .clone()
    }};
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    Search,

    Plus,
    Minus,

    Check,
    X,

    Settings,

    ChevronLeft,
    ChevronRight,
    ChevronDown,

    House,
    AppWindow,
    Download,
    HardDrive,

    Folder,
    FolderOpen,
    FolderPlus,

    File,
    FileText,
    FileImage,
    FileArchive,

    ExternalLink,

    LayoutList,
    LayoutGrid,
    Columns3,

    Eye,
    Volume2,
}

impl IconName {
    fn svg(self) -> SvgData {
        match self {
            Self::Search => {
                viewkit_svg!("search")
            }

            Self::Plus => {
                viewkit_svg!("plus")
            }

            Self::Minus => {
                viewkit_svg!("minus")
            }

            Self::Check => {
                viewkit_svg!("check")
            }

            Self::X => {
                viewkit_svg!("x")
            }

            Self::Settings => {
                viewkit_svg!("settings")
            }

            Self::ChevronLeft => {
                viewkit_svg!("chevron-left")
            }

            Self::ChevronRight => {
                viewkit_svg!("chevron-right")
            }

            Self::ChevronDown => {
                viewkit_svg!("chevron-down")
            }

            Self::House => {
                viewkit_svg!("house")
            }

            Self::AppWindow => {
                viewkit_svg!("app-window")
            }

            Self::Download => {
                viewkit_svg!("download")
            }

            Self::HardDrive => {
                viewkit_svg!("hard-drive")
            }

            Self::Folder => {
                viewkit_svg!("folder")
            }

            Self::FolderOpen => {
                viewkit_svg!("folder-open")
            }

            Self::FolderPlus => {
                viewkit_svg!("folder-plus")
            }

            Self::File => {
                viewkit_svg!("file")
            }

            Self::FileText => {
                viewkit_svg!("file-text")
            }

            Self::FileImage => {
                viewkit_svg!("file-image")
            }

            Self::FileArchive => {
                viewkit_svg!("file-archive")
            }

            Self::ExternalLink => {
                viewkit_svg!("external-link")
            }

            Self::LayoutList => {
                viewkit_svg!("layout-list")
            }

            Self::LayoutGrid => {
                viewkit_svg!("layout-grid")
            }

            Self::Columns3 => {
                viewkit_svg!("columns-3")
            }

            Self::Eye => {
                viewkit_svg!("eye")
            }

            Self::Volume2 => {
                viewkit_svg!("volume-2")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    name: IconName,

    size: f32,
    color: Color,
    opacity: f32,
}

impl Icon {
    pub const fn new(name: IconName) -> Self {
        Self {
            name,

            size: DEFAULT_ICON_SIZE,

            color: Color::from_rgb_hex(0x17181a),

            opacity: 1.0,
        }
    }

    pub const fn name(&self) -> IconName {
        self.name
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = sanitize_size(size);

        self
    }

    pub const fn color(mut self, color: Color) -> Self {
        self.color = color;

        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = sanitize_opacity(opacity);

        self
    }
}

impl View for Icon {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(self.size, self.size))
    }

    fn paint(&self, bounds: crate::geometry::Rect, context: &mut PaintContext<'_>) {
        Svg::new(self.name.svg())
            .tint(self.color)
            .opacity(self.opacity)
            .paint(bounds, context);
    }
}

fn sanitize_size(size: f32) -> f32 {
    if size.is_finite() && size > 0.0 {
        size
    } else {
        DEFAULT_ICON_SIZE
    }
}

fn sanitize_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
