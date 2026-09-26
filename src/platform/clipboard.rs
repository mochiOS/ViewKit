//! Internal text clipboard bridge used by editable controls.

#[cfg(target_os = "mochios")]
pub(crate) fn set_text(value: &str) -> bool {
    mochi_user_platform::workspace::set_clipboard_text(value).is_ok()
}

#[cfg(not(target_os = "mochios"))]
pub(crate) fn set_text(_value: &str) -> bool {
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
    None
}
