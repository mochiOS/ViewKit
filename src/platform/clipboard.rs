//! Internal text clipboard bridge used by editable controls.

#[cfg(target_os = "mochios")]
pub(crate) fn set_text(value: &str) -> bool {
    mochi_user_platform::workspace::set_clipboard_text(value).is_ok()
}

#[cfg(not(target_os = "mochios"))]
pub(crate) fn set_text(value: &str) -> bool {
    #[cfg(target_os = "linux")]
    if let Some(root) = std::env::var_os("MOCHIOS_PREVIEW_CONFIG_ROOT") {
        let root = std::path::PathBuf::from(root);
        return std::fs::create_dir_all(&root).is_ok()
            && std::fs::write(root.join("clipboard.txt"), value).is_ok();
    }
    let _ = value;
    false
}

#[cfg(target_os = "mochios")]
pub(crate) fn text() -> Option<String> {
    mochi_user_platform::workspace::clipboard_text()
        .ok()
        .flatten()
}

#[cfg(not(target_os = "mochios"))]
pub(crate) fn text() -> Option<String> {
    #[cfg(target_os = "linux")]
    if let Some(root) = std::env::var_os("MOCHIOS_PREVIEW_CONFIG_ROOT") {
        return std::fs::read_to_string(std::path::PathBuf::from(root).join("clipboard.txt")).ok();
    }
    None
}
