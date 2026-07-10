//! ViewKitで使用するフォントシステムを定義

pub(crate) use crate::platform::{DEFAULT_UI_FONT_FAMILY, load_system_fonts};
use cosmic_text::FontSystem;

const DEFAULT_UI_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_ui_font.ttf"));

pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    font_system
        .db_mut()
        .load_font_data(DEFAULT_UI_FONT_BYTES.to_vec());
    load_system_fonts(&mut font_system);

    font_system
        .db_mut()
        .set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);

    font_system
}
