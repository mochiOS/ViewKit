//! ViewKitで使用するフォントシステムを定義

use cosmic_text::FontSystem;
use cosmic_text::fontdb::{Family, Query, Stretch, Style, Weight};

const DEFAULT_UI_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_ui_font.ttf"));
pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = env!("VIEWKIT_DEFAULT_UI_FONT_FAMILY");

const FALLBACK_UI_FONT_FAMILIES: &[&str] = &[
    DEFAULT_UI_FONT_FAMILY,
    "Noto Sans CJK JP",
    "Noto Sans JP",
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
];

pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    font_system
        .db_mut()
        .load_font_data(DEFAULT_UI_FONT_BYTES.to_vec());

    let selected_family = FALLBACK_UI_FONT_FAMILIES
        .iter()
        .copied()
        .find(|family| font_family_exists(&font_system, family));

    let selected_family = selected_family.unwrap_or_else(|| {
        panic!(
            "ViewKit could not find a usable sans-serif font. \
             Install IBM Plex Sans JP, Noto Sans CJK JP, \
             Noto Sans, DejaVu Sans, or Liberation Sans."
        )
    });

    font_system.db_mut().set_sans_serif_family(selected_family);

    font_system
}

fn font_family_exists(font_system: &FontSystem, family: &str) -> bool {
    font_system
        .db()
        .query(&Query {
            families: &[Family::Name(family)],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        })
        .is_some()
}
