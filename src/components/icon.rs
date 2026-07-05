//! Iconコンポーネント

use super::Svg;
use crate::geometry::Size;
use crate::svg::SvgData;
use crate::theme::Color;
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::sync::OnceLock;

const DEFAULT_ICON_SIZE: f32 = 24.0;

macro_rules! lucide_svg {
    ($name:literal) => {{
        static DATA: OnceLock<SvgData> = OnceLock::new();

        DATA.get_or_init(|| {
            SvgData::decode(include_bytes!(concat!(
                "../../resources/icons/",
                $name,
                ".svg",
            )))
            .unwrap_or_else(|error| {
                panic!("アイコン `{}` を解析できませんでした: {}", $name, error,)
            })
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
                lucide_svg!("search")
            }

            Self::Plus => {
                lucide_svg!("plus")
            }

            Self::Minus => {
                lucide_svg!("minus")
            }

            Self::Check => {
                lucide_svg!("check")
            }

            Self::X => {
                lucide_svg!("x")
            }

            Self::Settings => {
                lucide_svg!("settings")
            }

            Self::ChevronLeft => {
                lucide_svg!("chevron-left")
            }

            Self::ChevronRight => {
                lucide_svg!("chevron-right")
            }

            Self::House => {
                lucide_svg!("house")
            }

            Self::AppWindow => {
                lucide_svg!("app-window")
            }

            Self::Download => {
                lucide_svg!("download")
            }

            Self::HardDrive => {
                lucide_svg!("hard-drive")
            }

            Self::Folder => {
                lucide_svg!("folder")
            }

            Self::FolderOpen => {
                lucide_svg!("folder-open")
            }

            Self::FolderPlus => {
                lucide_svg!("folder-plus")
            }

            Self::File => {
                lucide_svg!("file")
            }

            Self::FileText => {
                lucide_svg!("file-text")
            }

            Self::FileImage => {
                lucide_svg!("file-image")
            }

            Self::FileArchive => {
                lucide_svg!("file-archive")
            }

            Self::ExternalLink => {
                lucide_svg!("external-link")
            }

            Self::LayoutList => {
                lucide_svg!("layout-list")
            }

            Self::LayoutGrid => {
                lucide_svg!("layout-grid")
            }

            Self::Columns3 => {
                lucide_svg!("columns-3")
            }

            Self::Eye => {
                lucide_svg!("eye")
            }

            Self::Volume2 => {
                lucide_svg!("volume-2")
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

            color: Color::BLACK,

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
