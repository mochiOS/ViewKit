#!/usr/bin/env bash

set -euo pipefail

ROOT="$(
    cd "$(dirname "${BASH_SOURCE[0]}")/.."
    pwd
)"

cargo run \
    --quiet \
    --manifest-path "$ROOT/tools/ffi-gen/Cargo.toml" \
    -- \
    "$ROOT/src/components/mod.rs" \
    "$ROOT/src/ffi/generated_components.rs"

rustfmt \
    --edition 2024 \
    "$ROOT/src/ffi/generated_components.rs"

cbindgen \
    --config "$ROOT/cbindgen.toml" \
    --crate viewkit \
    --output "$ROOT/lib/include/viewkit_abi.h" \
    "$ROOT"

$ROOT/scripts/generate-header.sh