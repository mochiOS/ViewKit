# ViewKit

ViewKit provides the same Rust and C UI runtime on mochiOS, Linux, and Windows.
Linux and Windows use the shared winit/softbuffer desktop backend.

### Build

1. cbindgenをインストールします: `cargo install --force cbindgen` or `brew install cbindgen`
2. ライブラリをビルドします: `cargo build --release`

#### Run examples

`cargo run --example <example_name>`

### License

Please see the [LICENSE](license) file.
