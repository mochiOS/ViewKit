//! ViewKitで使用するフォントシステムを定義

use cosmic_text::FontSystem;

pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = "IBM Plex Sans JP";

pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();

    font_system
        .db_mut()
        .set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);

    font_system
}
