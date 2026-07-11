//! ViewKitで使用するフォントシステムを定義

pub(crate) use crate::platform::{DEFAULT_UI_FONT_FAMILY, load_platform_fonts};
use cosmic_text::{FontSystem, fontdb};

const DEFAULT_UI_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_ui_font.ttf"));

#[cfg(target_os = "mochios")]
pub(crate) fn create_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_data(DEFAULT_UI_FONT_BYTES.to_vec());
    load_platform_fonts(&mut db);
    db.set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);

    FontSystem::new_with_locale_and_db(String::from("en-US"), db)
}

#[cfg(not(target_os = "mochios"))]
pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    font_system
        .db_mut()
        .load_font_data(DEFAULT_UI_FONT_BYTES.to_vec());
    load_platform_fonts(font_system.db_mut());

    font_system
        .db_mut()
        .set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);

    font_system
}
