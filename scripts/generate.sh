#!/usr/bin/env bash

set -euo pipefail

ROOT="$(
    cd "$(dirname "${BASH_SOURCE[0]}")/.."
    pwd
)"

cargo run \
    --quiet \
    --manifest-path \
        "$ROOT/tools/ffi-gen/Cargo.toml" \
    -- \
    "$ROOT/src/components/mod.rs" \
    "$ROOT/src/ffi/generated_components.rs"

rustfmt \
    --edition 2024 \
    "$ROOT/src/ffi/generated_components.rs"

if ! command -v cbindgen >/dev/null 2>&1; then
    echo "error: cbindgen 0.28.0 or newer is required" >&2
    exit 1
fi

CBINDGEN_VERSION="$(cbindgen --version | awk '{print $2}')"
if [ "$(printf '%s\n' '0.28.0' "$CBINDGEN_VERSION" | sort -V | head -n 1)" != '0.28.0' ]; then
    echo "error: cbindgen $CBINDGEN_VERSION cannot parse Rust 2024 unsafe attributes; install cbindgen 0.28.0 or newer" >&2
    exit 1
fi

METADATA_FILE="$(mktemp)"
trap 'rm -f "$METADATA_FILE"' EXIT
cargo metadata \
    --manifest-path "$ROOT/Cargo.toml" \
    --format-version 1 \
    --no-deps \
    --offline \
    >"$METADATA_FILE"

(
    cd "$ROOT"

    cbindgen \
        --config cbindgen.toml \
        --crate viewkit \
        --metadata "$METADATA_FILE" \
        --output \
            lib/include/viewkit_abi.h
)
