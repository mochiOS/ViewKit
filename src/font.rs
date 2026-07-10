//! ViewKitで使用するフォントシステムを定義

use cosmic_text::FontSystem;

const DEFAULT_UI_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_ui_font.ttf"));
pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = env!("VIEWKIT_DEFAULT_UI_FONT_FAMILY");

pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    font_system
        .db_mut()
        .load_font_data(DEFAULT_UI_FONT_BYTES.to_vec());

    font_system
        .db_mut()
        .set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);

    font_system
}
