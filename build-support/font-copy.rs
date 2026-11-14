//! Build-time font staging without inheriting read-only SDK file permissions.
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

/// Keep unchanged font outputs intact and atomically replace changed outputs,
/// even when a previous build copied a read-only SDK font into OUT_DIR.
pub(crate) fn copy_font(source: &Path, destination: &Path) -> io::Result<()> {
    let bytes = fs::read(source)?;
    if fs::read(destination).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    let temporary = destination.with_extension(format!(
        "font-copy-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut output = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        output.write_all(&bytes)?;
        drop(output);
        fs::rename(&temporary, destination)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn read_only_inputs_and_existing_outputs_support_repeated_builds() {
        let root = std::env::temp_dir().join(format!("viewkit-font-copy-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let source = root.join("source.ttf");
        let destination = root.join("embedded.ttf");
        fs::write(&source, b"first font").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o444)).unwrap();
        copy_font(&source, &destination).unwrap();
        assert!(!fs::metadata(&destination).unwrap().permissions().readonly());
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o444)).unwrap();
        let modified = fs::metadata(&destination).unwrap().modified().unwrap();
        copy_font(&source, &destination).unwrap();
        assert_eq!(modified, fs::metadata(&destination).unwrap().modified().unwrap());
        let replacement = root.join("replacement.ttf");
        fs::write(&replacement, b"second font").unwrap();
        fs::set_permissions(&replacement, fs::Permissions::from_mode(0o444)).unwrap();
        copy_font(&replacement, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"second font");
        assert!(!fs::metadata(&destination).unwrap().permissions().readonly());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 3);
        fs::remove_dir_all(root).unwrap();
    }
}
