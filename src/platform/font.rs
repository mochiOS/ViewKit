#[cfg(target_os = "linux")]
pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = env!("VIEWKIT_DEFAULT_UI_FONT_FAMILY");

#[cfg(target_os = "linux")]
pub(crate) fn load_system_fonts(_font_system: &mut cosmic_text::FontSystem) {}

#[cfg(target_os = "mochios")]
pub(crate) const DEFAULT_UI_FONT_FAMILY: &str = "IBM Plex Sans JP";

#[cfg(target_os = "mochios")]
pub(crate) fn load_system_fonts(font_system: &mut cosmic_text::FontSystem) {
    use mochi_user_syscall as syscall;
    use std::ffi::CString;
    use std::string::String;
    use std::vec::Vec;

    const FONT_DIR: &str = "/libraries/fonts";
    const READ_BUF_SIZE: usize = 1024;

    fn c_path(path: &str) -> Result<CString, syscall::SysError> {
        CString::new(path).map_err(|_| syscall::SysError::from_raw(syscall::EINVAL as i64))
    }

    fn open_path(path: &str, flags: u64) -> Result<u64, syscall::SysError> {
        let path = c_path(path)?;
        syscall::call2(
            syscall::SyscallNumber::FileOpen,
            path.as_ptr() as u64,
            flags,
        )
    }

    fn close(fd: u64) {
        let _ = syscall::call1(syscall::SyscallNumber::FileClose, fd);
    }

    fn read_to_end_path(path: &str) -> Result<Vec<u8>, syscall::SysError> {
        let fd = open_path(path, 0)?;
        let mut out = Vec::new();
        let mut buf = [0u8; READ_BUF_SIZE];

        loop {
            let read = match syscall::call3(
                syscall::SyscallNumber::FileRead,
                fd,
                buf.as_mut_ptr() as u64,
                buf.len() as u64,
            ) {
                Ok(read) => read,
                Err(err) => {
                    close(fd);
                    return Err(err);
                }
            };

            if read == 0 {
                break;
            }

            let read = read as usize;
            out.extend_from_slice(&buf[..read]);

            if read < buf.len() {
                break;
            }
        }

        close(fd);
        Ok(out)
    }

    fn read_dir_names(path: &str) -> Result<Vec<String>, syscall::SysError> {
        let fd = open_path(path, 0)?;
        let mut out = Vec::new();
        let mut buf = [0u8; 1024];

        loop {
            let read = match syscall::call3(
                syscall::SyscallNumber::FileReadDir,
                fd,
                buf.as_mut_ptr() as u64,
                buf.len() as u64,
            ) {
                Ok(read) => read,
                Err(err) => {
                    close(fd);
                    return Err(err);
                }
            };

            if read == 0 {
                break;
            }

            let mut offset = 0usize;
            let read = read as usize;
            while offset + 19 <= read {
                let reclen = u16::from_ne_bytes([buf[offset + 16], buf[offset + 17]]) as usize;
                if reclen == 0 || offset + reclen > read {
                    break;
                }
                let name_start = offset + 19;
                let name_end = offset + reclen;
                let name_bytes = &buf[name_start..name_end];
                let name_len = name_bytes
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(name_bytes.len());
                if name_len > 0
                    && let Ok(name) = core::str::from_utf8(&name_bytes[..name_len])
                    && name != "."
                    && name != ".."
                {
                    out.push(name.to_string());
                }
                offset += reclen;
            }

            if read < buf.len() {
                break;
            }
        }

        close(fd);
        Ok(out)
    }

    let mut entries = read_dir_names(FONT_DIR)
        .unwrap_or_else(|err| panic!("failed to read font directory {FONT_DIR}: {err:?}"));
    entries.sort();

    let mut loaded = 0usize;
    for entry in entries {
        if !entry.ends_with(".ttf") {
            continue;
        }

        let path = format!("{FONT_DIR}/{entry}");
        let bytes = read_to_end_path(&path)
            .unwrap_or_else(|err| panic!("failed to load font {path}: {err:?}"));
        font_system.db_mut().load_font_data(bytes);
        loaded += 1;
    }

    assert!(loaded > 0, "no font files found in {}", FONT_DIR);
}
