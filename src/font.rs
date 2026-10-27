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
    load_additional_fonts(&mut db);
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
    load_additional_fonts(font_system.db_mut());
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

/// Registry changes and a bounded history of exact-file previews. Normal
/// typography does not trigger filesystem reads or new per-frame font parsing.
pub(crate) struct FontWatcher {
    revision: (u64, u64),
    previews: std::collections::BTreeMap<String, u64>,
}
impl FontWatcher {
    pub(crate) fn new() -> Self {
        Self {
            revision: mochios_fonts::revision(),
            previews: Default::default(),
        }
    }
    pub(crate) fn revision_changed(&mut self) -> bool {
        let revision = mochios_fonts::revision();
        if revision == self.revision {
            return false;
        }
        self.revision = revision;
        self.previews.clear();
        true
    }
    pub(crate) fn family_changed(&mut self, family: &str) -> bool {
        if !family.starts_with("mochi-font:") || self.previews.contains_key(family) {
            return false;
        }
        let size =
            mochios_fonts::preview_size(&mochios_fonts::Locations::current(), family).unwrap_or(0);
        let reset = self.previews.len() >= 8
            || self.previews.values().sum::<u64>().saturating_add(size) > 128 * 1024 * 1024;
        if reset {
            self.previews.clear();
        }
        self.previews.insert(family.into(), size);
        reset
    }
    pub(crate) fn display_changed(&mut self, list: &crate::draw_command::DisplayList) -> bool {
        let mut changed = self.revision_changed();
        for command in list.commands() {
            if let crate::draw_command::DrawCommand::DrawText { command } = command {
                changed |= self.family_changed(&command.font_family);
            }
        }
        changed
    }
}
fn load_additional_fonts(db: &mut fontdb::Database) {
    let mut locations = mochios_fonts::Locations::current();
    locations.system.clear();
    load_additional_fonts_at(db, &locations);
}
fn load_additional_fonts_at(db: &mut fontdb::Database, locations: &mochios_fonts::Locations) {
    for font in mochios_fonts::catalogue(locations)
        .files
        .into_iter()
        .filter(|f| f.enabled)
    {
        let previous: std::collections::HashSet<_> = db.faces().map(|face| face.id).collect();
        if db.load_font_file(&font.path).is_err() {
            continue;
        }
        // User fonts must not replace the established OS UI/IME families.
        let protected = db
            .faces()
            .filter(|face| {
                !previous.contains(&face.id)
                    && face.families.iter().any(|(name, _)| {
                        name == DEFAULT_UI_FONT_FAMILY
                            || name == DEFAULT_MONOSPACE_FONT_FAMILY
                            || JAPANESE_FAMILIES.contains(&name.as_str())
                    })
            })
            .map(|face| face.id)
            .collect::<Vec<_>>();
        for id in protected {
            db.remove_face(id);
        }
    }
}
pub(crate) fn ensure_requested_face(system: &mut FontSystem, family: &str) {
    if !family.starts_with("mochi-font:")
        || system
            .db()
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == family))
    {
        return;
    }
    let Ok(Some((bytes, index))) =
        mochios_fonts::preview_data(&mochios_fonts::Locations::current(), family)
    else {
        return;
    };
    let db = system.db_mut();
    let ids = db.load_font_source(fontdb::Source::Binary(std::sync::Arc::new(bytes)));
    let selected = ids
        .iter()
        .find_map(|id| db.face(*id).filter(|face| face.index == index).cloned());
    for id in ids {
        db.remove_face(id);
    }
    if let Some(mut face) = selected {
        let language = face
            .families
            .first()
            .map(|family| family.1)
            .unwrap_or(fontdb::Language::English_UnitedStates);
        face.families = vec![(family.into(), language)];
        db.push_face_info(face);
    }
}

#[cfg(test)]
mod installed_font_tests {
    use super::*;
    #[test]
    fn installed_names_cannot_replace_the_established_ui_family() {
        let current = mochios_fonts::Locations::current();
        let file = mochios_fonts::catalogue(&current)
            .files
            .into_iter()
            .find(|file| file.faces.iter().any(|face| face.family == "Inter"))
            .unwrap();
        let root = std::env::temp_dir().join(format!(
            "mochios-protected-font-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        std::fs::copy(file.path, root.join("custom.ttf")).unwrap();
        let mut system = create_font_system();
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(DEFAULT_UI_FONT_FAMILY)],
            ..Default::default()
        };
        let before = system.db().query(&query);
        load_additional_fonts_at(
            system.db_mut(),
            &mochios_fonts::Locations {
                system: vec![],
                shared: vec![],
                user: Some(root),
            },
        );
        assert_eq!(system.db().query(&query), before);
    }
    #[test]
    fn exact_preview_uses_the_selected_font_bytes_and_preserves_the_ui_face() {
        let locations = mochios_fonts::Locations::current();
        let file = mochios_fonts::catalogue(&locations)
            .files
            .into_iter()
            .find(|file| file.faces.iter().any(|face| face.family == "Inter"))
            .expect("test environment supplies Inter through MOCHIOS_PREVIEW_FONTS_SYSTEM");
        let mut system = create_font_system();
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(DEFAULT_UI_FONT_FAMILY)],
            ..Default::default()
        };
        let ui_before = system.db().query(&query);
        let alias = file.preview_family(file.faces[0].index);
        ensure_requested_face(&mut system, &alias);
        let id = system
            .db()
            .query(&fontdb::Query {
                families: &[fontdb::Family::Name(&alias)],
                ..Default::default()
            })
            .unwrap();
        system
            .db()
            .with_face_data(id, |data, index| {
                assert_eq!(index, file.faces[0].index);
                assert_eq!(data, std::fs::read(&file.path).unwrap());
            })
            .unwrap();
        assert_eq!(system.db().query(&query), ui_before);
    }
    #[test]
    fn exact_preview_history_is_bounded_and_plain_families_do_not_reset_it() {
        let mut watcher = FontWatcher::new();
        for i in 0..8 {
            assert!(!watcher.family_changed(&format!("mochi-font:invalid-{i}")));
        }
        assert!(!watcher.family_changed(DEFAULT_UI_FONT_FAMILY));
        assert!(watcher.family_changed("mochi-font:invalid-next"));
        assert_eq!(watcher.previews.len(), 1);
    }
}
