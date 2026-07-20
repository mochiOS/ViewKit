use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct FontCandidate {
    path: PathBuf,
    family: &'static str,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(target_os, values(\"mochios\"))");

    let target_os = env::var("CARGO_CFG_TARGET_OS")
        .expect("CARGO_CFG_TARGET_OS is not set");

    let candidates = font_candidates(&target_os);

    let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.path.exists())
    else {
        panic!("no usable system font found for ViewKit on {target_os}");
    };

    let out_dir = env::var("OUT_DIR")
        .expect("OUT_DIR is not set");

    let target_path = Path::new(&out_dir)
        .join("default_ui_font.ttf");

    fs::copy(&candidate.path, &target_path)
        .unwrap_or_else(|err| {
            panic!(
                "failed to copy default font from {}: {err}",
                candidate.path.display()
            )
        });

    println!(
        "cargo:rustc-env=VIEWKIT_DEFAULT_UI_FONT_FAMILY={}",
        candidate.family
    );
}

fn font_candidates(target_os: &str) -> Vec<FontCandidate> {
    match target_os {
        "windows" => windows_font_candidates(),
        "linux" => linux_font_candidates(),
        _ => Vec::new(),
    }
}

fn windows_font_candidates() -> Vec<FontCandidate> {
    let windows_dir = env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));

    let fonts_dir = windows_dir.join("Fonts");

    vec![
        FontCandidate {
            path: fonts_dir.join("segoeui.ttf"),
            family: "Segoe UI",
        },
        FontCandidate {
            path: fonts_dir.join("YuGothR.ttc"),
            family: "Yu Gothic",
        },
        FontCandidate {
            path: fonts_dir.join("meiryo.ttc"),
            family: "Meiryo",
        },
        FontCandidate {
            path: fonts_dir.join("arial.ttf"),
            family: "Arial",
        },
    ]
}

fn linux_font_candidates() -> Vec<FontCandidate> {
    vec![
        FontCandidate {
            path: PathBuf::from(
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            ),
            family: "DejaVu Sans",
        },
        FontCandidate {
            path: PathBuf::from(
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            ),
            family: "Noto Sans",
        },
        FontCandidate {
            path: PathBuf::from(
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            ),
            family: "Liberation Sans",
        },
    ]
}