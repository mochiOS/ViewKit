use std::env;
use std::fs;
use std::path::Path;

struct FontCandidate {
    path: &'static str,
    family: &'static str,
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

    let Some(candidate) = candidates
        .iter()
        .find(|candidate| Path::new(candidate.path).exists())
    else {
        panic!("no usable system font found for ViewKit");
    };

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is not set");
    let target_path = Path::new(&out_dir).join("default_ui_font.ttf");
    fs::copy(candidate.path, &target_path)
        .unwrap_or_else(|err| panic!("failed to copy default font: {err}"));

    println!(
        "cargo:rustc-env=VIEWKIT_DEFAULT_UI_FONT_FAMILY={}",
        candidate.family
    );
}
