# ViewKit

ViewKit provides the same Rust and C UI runtime on mochiOS, Linux, and Windows.
The repository is self-contained for desktop builds. Linux uses the shared
`winit`/`softbuffer` backend, while Windows uses a `winit` backend with a GPU
renderer powered by `wgpu`/DirectX 12 and falls back to `softbuffer` when a
display list command is not implemented by the GPU path yet. Linux and Windows
use fonts installed on the host system. Linux supports both Wayland and X11.

## Use from Rust

Add ViewKit directly from its Git repository:

```toml
[dependencies]
viewkit = { git = "https://github.com/mochiOS/ViewKit.git" }
```

Then create an application:

```rust
use viewkit::prelude::*;

struct Hello;

impl App for Hello {
    type Body = Text;

    fn new() -> Self {
        Self
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("Hello ViewKit").size(640.0, 400.0)
    }

    fn body(&self, _context: &ViewContext) -> Self::Body {
        Text::new("Hello from ViewKit")
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<Hello>()
}
```

Run it normally on Linux or Windows:

```sh
cargo run
```

## Build this repository

```sh
cargo build --release
cargo run --example button
```

Generating the C header additionally requires `cbindgen`:

```sh
cargo install --force cbindgen
./scripts/generate.sh
```

## VK Symbols

ViewKit embeds its standard monochrome symbols from `resources/symbols`. Each
SVG filename is its stable, lowercase, dot-separated identifier, such as
`chevron.left.svg` or `folder.fill.svg`. Symbols use a 24 by 24 canvas and are
automatically validated and exposed as `SymbolName` variants during the build.

```rust
Icon::new(SymbolName::Search)
IconButton::new(SymbolName::ChevronLeft)
    .accessibility_label("Back")
```

`IconName` remains as a deprecated compatibility alias. New code should use
`SymbolName`.

## mochiOS builds

mochiOS has no system font service, so its build supplies font files explicitly
through `VIEWKIT_UI_FONT_PATH` and `VIEWKIT_MONOSPACE_FONT_PATH`. Desktop users do
not need these variables or the surrounding mochiOS source tree.

## License

Please see the [LICENSE](license) file.
