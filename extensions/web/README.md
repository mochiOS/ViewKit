# ViewKit Web (unreleased API foundation)

An optional Rust crate in the ViewKit repository. The base `viewkit` crate does
not depend on this crate or WebKit. AppCore and Terra are unchanged.

**The native engine adapter is not connected. `WebSession::new` returns
`WebError::Unavailable`. This is not a working web renderer or browser engine.**
The test backend is compiled only for unit tests; it cannot be selected by an
application or silently substituted for a real engine.

The current implementation establishes session/page/presentation ownership,
asynchronous command acceptance, navigation identity, bounded UI notification
delivery, subscription lifetime and the ordinary ViewKit View wrapper. See
[the integration contract](../../docs/web-content.md) before implementing the
native adapter or the Kome ABI.

```toml
[dependencies]
viewkit-web = { path = "path/to/ViewKit/extensions/web" }
```

```sh
cargo check --offline --manifest-path extensions/web/Cargo.toml
cargo test --offline --manifest-path extensions/web/Cargo.toml
cargo doc --offline --no-deps --manifest-path extensions/web/Cargo.toml
```

All public APIs have Japanese reference comments. They are unreleased; no stable
ABI, SDK version or Kome API is implied. This crate does not ship an engine,
install a system library, grant capabilities or change a browser's UI.
