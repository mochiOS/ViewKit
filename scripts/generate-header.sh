#!/usr/bin/env bash

set -euo pipefail

ROOT="$(
    cd "$(dirname "${BASH_SOURCE[0]}")/.."
    pwd
)"

cd "$ROOT"

mkdir -p lib/include

cbindgen \
    --config cbindgen.toml \
    --crate viewkit \
    --output lib/include/viewkit_abi.h