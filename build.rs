use std::env;
use std::fs;
use std::path::PathBuf;

struct FontCandidate {
    path: PathBuf,
    family: String,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=VIEWKIT_UI_FONT_PATH");
    println!("cargo:rerun-if-env-changed=VIEWKIT_MONOSPACE_FONT_PATH");
    println!("cargo:rerun-if-env-changed=VIEWKIT_UI_FONT_FAMILY");
    println!("cargo:rerun-if-env-changed=VIEWKIT_MONOSPACE_FONT_FAMILY");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("mochios") {
        return;
    }

    let candidate = required_font("VIEWKIT_UI_FONT_PATH", "Inter Variable");
    let monospace = required_font("VIEWKIT_MONOSPACE_FONT_PATH", "UDEV Gothic");

    let Some(out_dir) = env::var_os("OUT_DIR") else {
        panic!("OUT_DIR is not set");
    };
    let out_dir = PathBuf::from(out_dir);
    let target_path = out_dir.join("default_ui_font.ttf");
    fs::copy(&candidate.path, &target_path)
        .unwrap_or_else(|err| panic!("failed to copy default font: {err}"));
    let monospace_target_path = out_dir.join("default_monospace_font.ttf");
    fs::copy(&monospace.path, &monospace_target_path)
        .unwrap_or_else(|err| panic!("failed to copy default monospace font: {err}"));

    println!(
        "cargo:rustc-env=VIEWKIT_DEFAULT_UI_FONT_FAMILY={}",
        candidate.family
    );
    println!(
        "cargo:rustc-env=VIEWKIT_DEFAULT_MONOSPACE_FONT_FAMILY={}",
        monospace.family
    );
}

fn required_font(path_variable: &str, default_family: &'static str) -> FontCandidate {
    let Some(path) = env::var_os(path_variable) else {
        panic!("{path_variable} must point to a font file when building ViewKit for mochiOS");
    };
    let candidate = FontCandidate {
        path: PathBuf::from(path),
        family: (match path_variable {
            "VIEWKIT_UI_FONT_PATH" => env::var("VIEWKIT_UI_FONT_FAMILY"),
            _ => env::var("VIEWKIT_MONOSPACE_FONT_FAMILY"),
        })
        .unwrap_or_else(|_| default_family.to_owned()),
    };
    println!("cargo:rerun-if-changed={}", candidate.path.display());
    if !candidate.path.is_file() {
        panic!("{path_variable} does not point to a readable font file");
    }
    candidate
}
