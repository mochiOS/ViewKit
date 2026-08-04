use std::env;
use std::fs;
use std::path::PathBuf;

struct FontCandidate {
    path: PathBuf,
    family: &'static str,
}

fn select_font<'a>(candidates: &'a [FontCandidate]) -> Option<&'a FontCandidate> {
    candidates.iter().find(|candidate| candidate.path.exists())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let Some(manifest_dir) = env::var_os("CARGO_MANIFEST_DIR") else {
        panic!("CARGO_MANIFEST_DIR is not set");
    };
    let manifest_dir = PathBuf::from(manifest_dir);
    let repository_fonts = manifest_dir.join("../fonts/out/fonts");
    let repository_ui_font = repository_fonts.join("InterVariable.ttf");
    let repository_monospace_font = repository_fonts.join("UDEVGothic-Regular.ttf");
    println!("cargo:rerun-if-changed={}", repository_ui_font.display());
    println!(
        "cargo:rerun-if-changed={}",
        repository_monospace_font.display()
    );

    let candidates = [
        FontCandidate {
            path: repository_ui_font,
            family: "Inter Variable",
        },
        FontCandidate {
            path: PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
            family: "DejaVu Sans",
        },
        FontCandidate {
            path: PathBuf::from("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
            family: "Noto Sans",
        },
        FontCandidate {
            path: PathBuf::from("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"),
            family: "Liberation Sans",
        },
    ];

    let Some(candidate) = select_font(&candidates) else {
        panic!("no usable font found for ViewKit; run `make fonts` from the mochiOS root");
    };

    let monospace_candidates = [
        FontCandidate {
            path: repository_monospace_font,
            family: "UDEV Gothic",
        },
        FontCandidate {
            path: PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"),
            family: "DejaVu Sans Mono",
        },
        FontCandidate {
            path: PathBuf::from("/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf"),
            family: "Liberation Mono",
        },
        FontCandidate {
            path: PathBuf::from("/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf"),
            family: "Liberation Mono",
        },
    ];
    let monospace = select_font(&monospace_candidates).unwrap_or(candidate);

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
