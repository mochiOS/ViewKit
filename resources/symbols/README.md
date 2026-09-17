# VK Symbols

This directory is the source of truth for ViewKit's standard monochrome UI
symbols.

- Use an SVG canvas of `24x24` with `viewBox="0 0 24 24"`.
- Use lowercase, dot-separated filenames such as `arrow.left.svg`.
- Keep the background transparent and let ViewKit apply the theme tint.
- Do not embed scripts, bitmap images, external resources, or text.
- Use `.fill` as the final name segment for filled variants.
- Do not rename a published symbol without providing a compatibility path.

`build.rs` validates and embeds every SVG and generates the Rust `SymbolName`
API. Applications must not load files from this directory at runtime.
