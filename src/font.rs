//! ViewKitで使用するフォントシステムを定義

pub(crate) use crate::platform::{
    DEFAULT_MONOSPACE_FONT_FAMILY, DEFAULT_UI_FONT_FAMILY, load_platform_fonts,
};
#[cfg(target_os = "mochios")]
use cosmic_text::fontdb;
use cosmic_text::{Family, FontSystem};

#[cfg(target_os = "mochios")]
const DEFAULT_UI_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_ui_font.ttf"));
#[cfg(target_os = "mochios")]
const DEFAULT_MONOSPACE_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_monospace_font.ttf"));

pub(crate) fn resolve_font_family(name: &str) -> Family<'_> {
    match name {
        "sans-serif" => Family::SansSerif,
        "monospace" => Family::Monospace,
        _ => Family::Name(name),
    }
}

#[cfg(target_os = "mochios")]
pub(crate) fn create_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_data(DEFAULT_UI_FONT_BYTES.to_vec());
    db.load_font_data(DEFAULT_MONOSPACE_FONT_BYTES.to_vec());
    load_platform_fonts(&mut db);
    db.set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);
    db.set_monospace_family(DEFAULT_MONOSPACE_FONT_FAMILY);

    FontSystem::new_with_locale_and_db(String::from("en-US"), db)
}

#[cfg(not(target_os = "mochios"))]
pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    load_platform_fonts(font_system.db_mut());
    font_system
}
