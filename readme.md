# ViewKit

ViewKit provides the same Rust and C UI runtime on mochiOS, Linux, and Windows.
The repository is self-contained for desktop builds: Linux and Windows use the
shared `winit`/`softbuffer` backend and fonts installed on the host system.
Linux supports both Wayland and X11.

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

## mochiOS builds

mochiOS has no system font service, so its build supplies font files explicitly
through `VIEWKIT_UI_FONT_PATH` and `VIEWKIT_MONOSPACE_FONT_PATH`. Desktop users do
not need these variables or the surrounding mochiOS source tree.

## License

Please see the [LICENSE](license) file.
