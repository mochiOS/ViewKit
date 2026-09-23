use std::env;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

struct FontCandidate {
    path: PathBuf,
    family: String,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let token_dir = Path::new("resources/var");
    for file in [
        "Light.tokens.json",
        "Dark.tokens.json",
        "Spacing.tokens.json",
        "Shape.tokens.json",
        "Typography.tokens.json",
        "Layout.tokens.json",
    ] {
        println!("cargo:rerun-if-changed={}", token_dir.join(file).display());
    }

    let Some(out_dir) = env::var_os("OUT_DIR") else {
        panic!("OUT_DIR is not set");
    };
    let out_dir = PathBuf::from(out_dir);
    generate_figma_tokens(token_dir, &out_dir);
    generate_symbols(Path::new("resources/symbols"), &out_dir);

    println!("cargo:rerun-if-env-changed=VIEWKIT_UI_FONT_PATH");
    println!("cargo:rerun-if-env-changed=VIEWKIT_MONOSPACE_FONT_PATH");
    println!("cargo:rerun-if-env-changed=VIEWKIT_JAPANESE_FONT_PATH");
    println!("cargo:rerun-if-env-changed=VIEWKIT_UI_FONT_FAMILY");
    println!("cargo:rerun-if-env-changed=VIEWKIT_MONOSPACE_FONT_FAMILY");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("mochios") {
        return;
    }

    let candidate = required_font("VIEWKIT_UI_FONT_PATH", "Inter Variable");
    let monospace = required_font("VIEWKIT_MONOSPACE_FONT_PATH", "UDEV Gothic");
    let japanese = required_font("VIEWKIT_JAPANESE_FONT_PATH", "IBM Plex Sans JP");

    let target_path = out_dir.join("default_ui_font.ttf");
    fs::copy(&candidate.path, &target_path)
        .unwrap_or_else(|err| panic!("failed to copy default font: {err}"));
    let monospace_target_path = out_dir.join("default_monospace_font.ttf");
    fs::copy(&monospace.path, &monospace_target_path)
        .unwrap_or_else(|err| panic!("failed to copy default monospace font: {err}"));
    fs::copy(&japanese.path, out_dir.join("default_japanese_font.ttf"))
        .unwrap_or_else(|err| panic!("failed to copy Japanese fallback font: {err}"));

    println!(
        "cargo:rustc-env=VIEWKIT_DEFAULT_UI_FONT_FAMILY={}",
        candidate.family
    );
    println!(
        "cargo:rustc-env=VIEWKIT_DEFAULT_MONOSPACE_FONT_FAMILY={}",
        monospace.family
    );
}

fn generate_symbols(symbol_dir: &Path, out_dir: &Path) {
    println!("cargo:rerun-if-changed={}", symbol_dir.display());

    let mut assets = BTreeMap::new();
    let mut variants = BTreeSet::new();
    let entries = fs::read_dir(symbol_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", symbol_dir.display()));

    for entry in entries {
        let path = entry.expect("failed to read symbol directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("svg") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("symbol filename is not UTF-8: {}", path.display()));
        validate_symbol_name(name, &path);
        validate_symbol_svg(&path);

        let variant = symbol_variant(name);
        if !variants.insert(variant.clone()) {
            panic!("symbol names generate the same Rust variant: {variant}");
        }
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("validated symbol filename")
            .to_owned();
        assets.insert(name.to_owned(), (variant, filename));
    }

    if assets.is_empty() {
        panic!("resources/symbols must contain at least one SVG symbol");
    }

    let public_assets = assets
        .iter()
        .map(|(name, (variant, filename))| (name.clone(), variant.clone(), Some(filename.clone())))
        .collect::<Vec<_>>();
    let mut symbols = public_assets.clone();

    // Keep source compatibility while applications migrate from IconName. Entries
    // without an SVG remain invisible until the corresponding VK Symbol is added.
    for (variant, name, replacement) in [
        ("Plus", "plus", Some("plus")),
        ("Minus", "minus", Some("minus")),
        ("Check", "check", None),
        ("X", "x", Some("x")),
        ("Settings", "settings", None),
        ("ArrowUp", "arrow.top", Some("arrow.top")),
        ("House", "home", Some("home")),
        ("AppWindow", "app.window", None),
        ("Download", "download", Some("download")),
        ("HardDrive", "hard.drive", Some("internaldrive")),
        ("FolderOpen", "folder.open", Some("folder.fill")),
        ("FolderPlus", "folder.plus", None),
        ("FileText", "file.text", None),
        ("FileImage", "file.image", None),
        ("FileArchive", "file.archive", None),
        ("ExternalLink", "external", Some("external")),
        ("LayoutList", "list", Some("list")),
        ("LayoutGrid", "grid", Some("grid")),
        ("Columns3", "columns.3", None),
        ("Eye", "eye", None),
        ("Volume2", "volume.2", None),
    ] {
        if variants.insert(variant.to_owned()) {
            let filename = replacement
                .and_then(|replacement| assets.get(replacement))
                .map(|(_, filename)| filename.clone());
            symbols.push((name.to_owned(), variant.to_owned(), filename));
        }
    }
    symbols.sort_by(|left, right| left.1.cmp(&right.1));

    let mut generated = String::from(
        "// @generated by build.rs from resources/symbols/*.svg.\n\
         // Add or remove SVG files instead of editing this file.\n\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n\
         pub enum SymbolName {\n",
    );
    for (_, variant, _) in &symbols {
        writeln!(generated, "    {variant},").expect("write symbol variant");
    }
    generated.push_str("}\n\nimpl SymbolName {\n    pub const ALL: &'static [Self] = &[\n");
    for (_, variant, _) in &public_assets {
        writeln!(generated, "        Self::{variant},").expect("write symbol list");
    }
    generated.push_str("    ];\n\n    pub const fn asset_name(self) -> &'static str {\n        match self {\n");
    for (name, variant, _) in &symbols {
        writeln!(generated, "            Self::{variant} => \"{name}\",")
            .expect("write symbol name");
    }
    generated.push_str("        }\n    }\n\n    pub fn from_asset_name(name: &str) -> Option<Self> {\n        match name {\n");
    for (name, variant, _) in &public_assets {
        writeln!(generated, "            \"{name}\" => Some(Self::{variant}),")
            .expect("write symbol lookup");
    }
    generated.push_str("            _ => None,\n        }\n    }\n\n    pub(crate) fn svg(self) -> Option<SvgData> {\n        match self {\n");
    for (index, (_, variant, filename)) in symbols.iter().enumerate() {
        if let Some(filename) = filename {
            writeln!(
                generated,
                "            Self::{variant} => {{\n\
                 static SYMBOL_{index}: OnceLock<Option<SvgData>> = OnceLock::new();\n\
                 SYMBOL_{index}.get_or_init(|| SvgData::decode(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/resources/symbols/{filename}\"))).ok()).clone()\n\
                 }},"
            )
            .expect("write embedded symbol");
        } else {
            writeln!(generated, "            Self::{variant} => None,")
                .expect("write unavailable compatibility symbol");
        }
    }
    generated.push_str("        }\n    }\n}\n");

    fs::write(out_dir.join("symbols.rs"), generated)
        .unwrap_or_else(|error| panic!("failed to write generated VK Symbols: {error}"));
}

fn validate_symbol_name(name: &str, path: &Path) {
    let valid = !name.is_empty()
        && name.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .enumerate()
                    .all(|(index, byte)| byte.is_ascii_lowercase() || byte.is_ascii_digit() && index > 0)
        });
    if !valid {
        panic!(
            "invalid VK Symbol name {}: expected lowercase dot-separated words",
            path.display()
        );
    }
}

fn validate_symbol_svg(path: &Path) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    for required in ["width=\"24\"", "height=\"24\"", "viewBox=\"0 0 24 24\""] {
        if !source.contains(required) {
            panic!("{} must use the 24x24 VK Symbols canvas", path.display());
        }
    }
    let lowercase = source.to_ascii_lowercase();
    for forbidden in ["<script", "<image", "<foreignobject", "xlink:href", " href="] {
        if lowercase.contains(forbidden) {
            panic!("{} contains forbidden SVG content: {forbidden}", path.display());
        }
    }
}

fn symbol_variant(name: &str) -> String {
    let mut variant = String::new();
    for segment in name.split('.') {
        let mut bytes = segment.bytes();
        let first = bytes.next().expect("validated symbol segment");
        variant.push(first.to_ascii_uppercase() as char);
        variant.extend(bytes.map(char::from));
    }
    variant
}

fn generate_figma_tokens(token_dir: &Path, out_dir: &Path) {
    let light = read_json(&token_dir.join("Light.tokens.json"));
    let dark = read_json(&token_dir.join("Dark.tokens.json"));
    let spacing = read_json(&token_dir.join("Spacing.tokens.json"));
    let shape = read_json(&token_dir.join("Shape.tokens.json"));
    let typography = read_json(&token_dir.join("Typography.tokens.json"));
    let layout = read_json(&token_dir.join("Layout.tokens.json"));

    let mut generated = String::from(
        "// @generated by build.rs from resources/var/*.tokens.json.\n\
         // Do not edit this file directly.\n\n",
    );
    emit_color_theme(&mut generated, "FIGMA_LIGHT_COLORS", &light);
    emit_color_theme(&mut generated, "FIGMA_DARK_COLORS", &dark);

    writeln!(
        generated,
        "pub const FIGMA_SPACING: FigmaSpacingTokens = FigmaSpacingTokens {{\n\
         space_2: {}, space_4: {}, space_8: {}, space_12: {}, space_16: {},\n\
         space_24: {}, space_32: {}, space_40: {}, space_48: {}, space_64: {},\n\
         }};",
        number(&spacing, &["Spacing", "2"]),
        number(&spacing, &["Spacing", "4"]),
        number(&spacing, &["Spacing", "8"]),
        number(&spacing, &["Spacing", "12"]),
        number(&spacing, &["Spacing", "16"]),
        number(&spacing, &["Spacing", "24"]),
        number(&spacing, &["Spacing", "32"]),
        number(&spacing, &["Spacing", "40"]),
        number(&spacing, &["Spacing", "48"]),
        number(&spacing, &["Spacing", "64"]),
    )
    .expect("write generated spacing tokens");

    writeln!(
        generated,
        "pub const FIGMA_SHAPE: FigmaShapeTokens = FigmaShapeTokens {{\n\
         radius_small: {}, radius_medium: {}, radius_large: {}, radius_xlarge: {},\n\
         stroke_default: {},\n\
         }};",
        number(&shape, &["Radius", "Small"]),
        number(&shape, &["Radius", "Medium"]),
        number(&shape, &["Radius", "Large"]),
        number(&shape, &["Radius", "XLarge"]),
        number(&shape, &["Stroke", "Default"]),
    )
    .expect("write generated shape tokens");

    writeln!(
        generated,
        "pub const FIGMA_TYPOGRAPHY: FigmaTypographyTokens = FigmaTypographyTokens {{\n\
         display_large: {}, display_medium: {}, title_large: {}, title_medium: {},\n\
         title_small: {}, body: {}, label: {}, caption: {},\n\
         }};",
        typography_expr(&typography, &["Typography", "Display", "Large"]),
        typography_expr(&typography, &["Typography", "Display", "Medium"]),
        typography_expr(&typography, &["Typography", "Title", "Large"]),
        typography_expr(&typography, &["Typography", "Title", "Medium"]),
        typography_expr(&typography, &["Typography", "Title", "Small"]),
        typography_expr(&typography, &["Typography", "Body"]),
        typography_expr(&typography, &["Typography", "Label"]),
        typography_expr(&typography, &["Typography", "Caption"]),
    )
    .expect("write generated typography tokens");

    writeln!(
        generated,
        "pub const FIGMA_LAYOUT: FigmaLayoutTokens = FigmaLayoutTokens {{\n\
         toolbar_height: {}, sidebar_width: {}, page_margin: {}, content_max_width: {},\n\
         form_width: {}, section_gap: {},\n\
         }};",
        number(&layout, &["Layout", "Toolbar", "Height"]),
        number(&layout, &["Layout", "Sidebar", "Width"]),
        number(&layout, &["Layout", "Page", "Margin"]),
        number(&layout, &["Layout", "Content", "MaxWidth"]),
        number(&layout, &["Layout", "Form", "Width"]),
        number(&layout, &["Layout", "Section", "Gap"]),
    )
    .expect("write generated layout tokens");

    fs::write(out_dir.join("figma_tokens.rs"), generated)
        .unwrap_or_else(|error| panic!("failed to write generated Figma tokens: {error}"));
}

fn emit_color_theme(output: &mut String, name: &str, root: &Value) {
    writeln!(
        output,
        "pub const {name}: FigmaColorTokens = FigmaColorTokens {{\n\
         accent: AccentColorTokens {{ primary: {}, hover: {}, pressed: {}, subtle: {}, on_accent: {} }},\n\
         background: BackgroundColorTokens {{ primary: {}, secondary: {}, elevated: {} }},\n\
         text: TextColorTokens {{ primary: {}, secondary: {}, tertiary: {}, disabled: {} }},\n\
         border: BorderColorTokens {{ default: {}, subtle: {}, strong: {} }},\n\
         semantic: SemanticColorTokens {{ warning: {}, success: {}, information: {}, error: ErrorColorTokens {{ base: {}, hover: {}, pressed: {}, disabled: {} }} }},\n\
         }};",
        color(root, &["Accent", "Primary"]),
        color(root, &["Accent", "Hover"]),
        color(root, &["Accent", "Pressed"]),
        color(root, &["Accent", "Subtle"]),
        color(root, &["Accent", "OnAccent"]),
        color(root, &["Background", "Primary"]),
        color(root, &["Background", "Secondary"]),
        color(root, &["Background", "Elevated"]),
        color(root, &["Text", "Primary"]),
        color(root, &["Text", "Secondary"]),
        color(root, &["Text", "Tertiary"]),
        color(root, &["Text", "Disabled"]),
        color(root, &["Border", "Default"]),
        color(root, &["Border", "Subtle"]),
        color(root, &["Border", "Strong"]),
        color(root, &["Semantic", "Warning"]),
        color(root, &["Semantic", "Success"]),
        color(root, &["Semantic", "Information"]),
        color(root, &["Semantic", "Error", "$root"]),
        color(root, &["Semantic", "Error", "Hover"]),
        color(root, &["Semantic", "Error", "Pressed"]),
        color(root, &["Semantic", "Error", "Disabled"]),
    )
    .expect("write generated color tokens");
}

fn read_json(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("invalid token JSON in {}: {error}", path.display()))
}

fn value_at<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
    let mut value = root;
    for segment in path {
        value = value.get(*segment).unwrap_or_else(|| {
            panic!("missing Figma token path {}", path.join("/"));
        });
    }
    value
}

fn number(root: &Value, path: &[&str]) -> String {
    let value = value_at(root, path)
        .get("$value")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("Figma token {} must be a number", path.join("/")));
    format!("{:?}_f32", value as f32)
}

fn integer(root: &Value, path: &[&str]) -> u16 {
    let value = value_at(root, path)
        .get("$value")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("Figma token {} must be an integer", path.join("/")));
    u16::try_from(value)
        .unwrap_or_else(|_| panic!("Figma token {} does not fit in u16", path.join("/")))
}

fn color(root: &Value, path: &[&str]) -> String {
    let token = value_at(root, path);
    let value = token
        .get("$value")
        .unwrap_or_else(|| panic!("Figma token {} has no $value", path.join("/")));
    let hex = value
        .get("hex")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("Figma token {} must contain a hex color", path.join("/")));
    let rgb = u32::from_str_radix(hex.trim_start_matches('#'), 16)
        .unwrap_or_else(|_| panic!("Figma token {} has an invalid hex color", path.join("/")));
    let alpha = value.get("alpha").and_then(Value::as_f64).unwrap_or(1.0);
    if !(0.0..=1.0).contains(&alpha) {
        panic!("Figma token {} has an invalid alpha", path.join("/"));
    }
    let alpha = (alpha * 255.0).round() as u32;
    format!("Color::from_rgba_hex(0x{:08X})", (rgb << 8) | alpha)
}

fn typography_expr(root: &Value, path: &[&str]) -> String {
    let mut font_size = path.to_vec();
    font_size.push("FontSize");
    let mut font_weight = path.to_vec();
    font_weight.push("FontWeight");
    let mut line_height = path.to_vec();
    line_height.push("LineHeight");
    format!(
        "TypographyToken {{ font_size: {}, font_weight: {}, line_height: {} }}",
        number(root, &font_size),
        integer(root, &font_weight),
        number(root, &line_height),
    )
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
