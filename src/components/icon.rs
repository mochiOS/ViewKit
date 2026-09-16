//! Iconコンポーネント
//!
//! アイコン資産はFigmaを正として再構築中です。公開APIは維持しますが、
//! 新しい資産が登録されるまではアイコンを描画しません。

use crate::geometry::Size;
use crate::theme::{Color, LayoutTokens};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

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
    ArrowUp,

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
    /// Stable asset name used by the icon rebuild plan.
    pub const fn asset_name(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Check => "check",
            Self::X => "x",
            Self::Settings => "settings",
            Self::ChevronLeft => "chevron-left",
            Self::ChevronRight => "chevron-right",
            Self::ChevronDown => "chevron-down",
            Self::ArrowUp => "arrow-up",
            Self::House => "house",
            Self::AppWindow => "app-window",
            Self::Download => "download",
            Self::HardDrive => "hard-drive",
            Self::Folder => "folder",
            Self::FolderOpen => "folder-open",
            Self::FolderPlus => "folder-plus",
            Self::File => "file",
            Self::FileText => "file-text",
            Self::FileImage => "file-image",
            Self::FileArchive => "file-archive",
            Self::ExternalLink => "external-link",
            Self::LayoutList => "layout-list",
            Self::LayoutGrid => "layout-grid",
            Self::Columns3 => "columns-3",
            Self::Eye => "eye",
            Self::Volume2 => "volume-2",
        }
    }

    pub(crate) const fn control_size(self, layout: LayoutTokens) -> f32 {
        match self {
            Self::ArrowUp => layout.compact_icon_size,
            _ => layout.control_icon_size,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    name: IconName,

    size: Option<f32>,
    color: Color,
    opacity: f32,
    accessibility_label: Option<String>,
}

impl Icon {
    pub const fn new(name: IconName) -> Self {
        Self {
            name,

            size: None,

            color: Color::from_rgb_hex(0x17181a),

            opacity: 1.0,

            accessibility_label: None,
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

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl View for Icon {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let size = self.size.unwrap_or(context.theme.layout.icon_button_size);
        constraints.constrain(Size::new(size, size))
    }

    fn paint(&self, _bounds: crate::geometry::Rect, _context: &mut PaintContext<'_>) {
        // Intentionally empty while the Figma-authored icon set is rebuilt.
        // See docs/viewkit/icons_plan.md.
    }
}

fn sanitize_size(size: f32) -> Option<f32> {
    if size.is_finite() && size > 0.0 {
        Some(size)
    } else {
        None
    }
}

fn sanitize_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
