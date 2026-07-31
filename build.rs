use std::env;
use std::fs;
use std::path::Path;

struct FontCandidate {
    path: &'static str,
    family: &'static str,
}

fn select_font<'a>(candidates: &'a [FontCandidate]) -> Option<&'a FontCandidate> {
    candidates
        .iter()
        .find(|candidate| Path::new(candidate.path).exists())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let candidates = [
        FontCandidate {
            path: "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            family: "DejaVu Sans",
        },
        FontCandidate {
            path: "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            family: "Noto Sans",
        },
        FontCandidate {
            path: "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            family: "Liberation Sans",
        },
    ];

    let Some(candidate) = select_font(&candidates) else {
        panic!("no usable system font found for ViewKit");
    };

    let monospace_candidates = [
        FontCandidate {
            path: "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            family: "DejaVu Sans Mono",
        },
        FontCandidate {
            path: "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
            family: "Liberation Mono",
        },
        FontCandidate {
            path: "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            family: "Liberation Mono",
        },
    ];
    let monospace = select_font(&monospace_candidates).unwrap_or(candidate);

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is not set");
    let target_path = Path::new(&out_dir).join("default_ui_font.ttf");
    fs::copy(candidate.path, &target_path)
        .unwrap_or_else(|err| panic!("failed to copy default font: {err}"));
    let monospace_target_path = Path::new(&out_dir).join("default_monospace_font.ttf");
    fs::copy(monospace.path, &monospace_target_path)
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
