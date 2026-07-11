#[cfg(target_os = "linux")]
pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = env!("VIEWKIT_DEFAULT_UI_FONT_FAMILY");

#[cfg(target_os = "linux")]
pub(crate) fn load_platform_fonts(_db: &mut cosmic_text::fontdb::Database) {}

#[cfg(target_os = "mochios")]
pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = env!("VIEWKIT_DEFAULT_UI_FONT_FAMILY");

#[cfg(target_os = "mochios")]
pub(crate) fn load_platform_fonts(_db: &mut cosmic_text::fontdb::Database) {}
