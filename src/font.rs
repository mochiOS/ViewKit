//! ViewKitで使用するフォントシステムを定義

pub(crate) use crate::platform::{
    DEFAULT_MONOSPACE_FONT_FAMILY, DEFAULT_UI_FONT_FAMILY, load_platform_fonts,
};
use cosmic_text::fontdb;
use cosmic_text::{Fallback, Family, FontSystem, PlatformFallback};
use unicode_script::Script;

#[cfg(target_os = "mochios")]
const DEFAULT_UI_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_ui_font.ttf"));
#[cfg(target_os = "mochios")]
const DEFAULT_MONOSPACE_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_monospace_font.ttf"));
#[cfg(viewkit_embedded_japanese_font)]
const DEFAULT_JAPANESE_FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/default_japanese_font.ttf"));

const JAPANESE_FAMILIES: &[&str] = &[
    env!("VIEWKIT_DEFAULT_JAPANESE_FONT_FAMILY"),
    "Noto Sans CJK JP",
    "Noto Sans JP",
    "Yu Gothic",
    "Meiryo",
    "Hiragino Sans",
];

struct UiFontFallback {
    common: Vec<&'static str>,
}
impl UiFontFallback {
    fn new() -> Self {
        Self {
            common: JAPANESE_FAMILIES
                .iter()
                .copied()
                .chain(PlatformFallback.common_fallback().iter().copied())
                .collect(),
        }
    }
}
impl Fallback for UiFontFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &self.common
    }
    fn forbidden_fallback(&self) -> &[&'static str] {
        PlatformFallback.forbidden_fallback()
    }
    fn script_fallback(&self, script: Script, locale: &str) -> &[&'static str] {
        match script {
            Script::Han | Script::Hiragana | Script::Katakana => JAPANESE_FAMILIES,
            _ => PlatformFallback.script_fallback(script, locale),
        }
    }
}

fn load_japanese_fonts(db: &mut fontdb::Database) {
    #[cfg(viewkit_embedded_japanese_font)]
    db.load_font_data(DEFAULT_JAPANESE_FONT_BYTES.to_vec());
    #[cfg(viewkit_japanese_medium)]
    db.load_font_data(include_bytes!(concat!(env!("OUT_DIR"), "/japanese_medium.ttf")).to_vec());
    #[cfg(viewkit_japanese_semibold)]
    db.load_font_data(include_bytes!(concat!(env!("OUT_DIR"), "/japanese_semibold.ttf")).to_vec());
    let _ = db;
}

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
    load_japanese_fonts(&mut db);
    load_platform_fonts(&mut db);
    db.set_sans_serif_family(DEFAULT_UI_FONT_FAMILY);
    db.set_monospace_family(DEFAULT_MONOSPACE_FONT_FAMILY);

    FontSystem::new_with_locale_and_db_and_fallback(
        String::from("ja-JP"),
        db,
        UiFontFallback::new(),
    )
}

#[cfg(not(target_os = "mochios"))]
pub(crate) fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    load_platform_fonts(font_system.db_mut());
    load_japanese_fonts(font_system.db_mut());
    let (locale, db) = font_system.into_locale_and_db();
    FontSystem::new_with_locale_and_db_and_fallback(locale, db, UiFontFallback::new())
}

#[cfg(all(
    test,
    viewkit_embedded_japanese_font,
    viewkit_japanese_medium,
    viewkit_japanese_semibold
))]
mod tests {
    use super::*;
    use cosmic_text::{Attrs, Buffer, Metrics, Shaping, Weight};

    #[test]
    fn ui_weights_use_gothic_japanese_and_keep_latin_in_inter() {
        let mut system = create_font_system();
        for weight in [400, 500, 600] {
            for text in [
                "あいうえお",
                "設定を変更",
                "アプリケーション",
                "Settings 設定",
                "「」、。",
            ] {
                let mut buffer = Buffer::new(&mut system, Metrics::new(14.0, 20.0));
                let glyphs = {
                    let mut buffer = buffer.borrow_with(&mut system);
                    buffer.set_size(Some(400.0), None);
                    buffer.set_text(
                        text,
                        &Attrs::new()
                            .family(resolve_font_family(DEFAULT_UI_FONT_FAMILY))
                            .weight(Weight(weight)),
                        Shaping::Advanced,
                        None,
                    );
                    buffer
                        .layout_runs()
                        .flat_map(|run| {
                            run.glyphs.iter().map(|glyph| {
                                (glyph.start, glyph.end, glyph.font_id, glyph.glyph_id)
                            })
                        })
                        .collect::<Vec<_>>()
                };
                assert!(!glyphs.is_empty());
                for (start, end, id, glyph) in glyphs {
                    let cluster = &text[start..end];
                    if cluster.trim().is_empty() {
                        continue;
                    }
                    let face = system.db().face(id).expect("selected font face");
                    let expected = if cluster.is_ascii() {
                        DEFAULT_UI_FONT_FAMILY
                    } else {
                        env!("VIEWKIT_DEFAULT_JAPANESE_FONT_FAMILY")
                    };
                    assert_ne!(glyph, 0, "missing glyph in {text}");
                    assert!(
                        face.families.iter().any(|(family, _)| family == expected),
                        "{text} / weight {weight} / {cluster}: {:?}",
                        face.families
                    );
                }
            }
        }
    }
}
