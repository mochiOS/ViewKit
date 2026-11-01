//! Shared text clipboard used by native controls and custom text surfaces.
//! Native access retains workspace Capability checks; Linux previews use the isolated session clipboard.

#[cfg(target_os = "mochios")]
/// UTF-8テキストを共有クリップボードへ書き込みます。成功した場合は`true`を返します。
/// mochiOSでは`clipboard.write`が必要です。Linuxではプレビューのセッション領域を使います。
pub fn set_text(value: &str) -> bool {
    mochi_user_platform::workspace::set_clipboard_text(value).is_ok()
}

#[cfg(not(target_os = "mochios"))]
/// UTF-8テキストを共有クリップボードへ書き込みます。成功した場合は`true`を返します。
/// mochiOSでは`clipboard.write`が必要です。Linuxではプレビューのセッション領域を使います。
pub fn set_text(value: &str) -> bool {
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
/// 共有クリップボードのUTF-8テキストを読み込みます。利用不可・非テキストの場合は`None`です。
/// mochiOSでは`clipboard.read`が必要です。Linuxではプレビューのセッション領域を使います。
pub fn text() -> Option<String> {
    mochi_user_platform::workspace::clipboard_text()
        .ok()
        .flatten()
}

#[cfg(not(target_os = "mochios"))]
/// 共有クリップボードのUTF-8テキストを読み込みます。利用不可・非テキストの場合は`None`です。
/// mochiOSでは`clipboard.read`が必要です。Linuxではプレビューのセッション領域を使います。
pub fn text() -> Option<String> {
    #[cfg(target_os = "linux")]
    if let Some(root) = std::env::var_os("MOCHIOS_PREVIEW_CONFIG_ROOT") {
        return std::fs::read_to_string(std::path::PathBuf::from(root).join("clipboard.txt")).ok();
    }
    None
}
