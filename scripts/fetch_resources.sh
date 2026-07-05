#!/usr/bin/env bash

set -euo pipefail

ROOT="$(
    cd "$(
        dirname "${BASH_SOURCE[0]}"
    )/.."
    pwd
)"

DESTINATION="${ROOT}/resources/icons"
ICON_DESTINATION="${DESTINATION}"

BASE_URL="https://raw.githubusercontent.com/lucide-icons/lucide/main"

ICONS=(
    search
    plus
    minus
    check
    x
    settings
    chevron-left
    chevron-right
)

mkdir -p "${ICON_DESTINATION}"

for icon in "${ICONS[@]}"; do
    echo "fetching ${icon}.svg"

    curl \
        --fail \
        --location \
        --silent \
        --show-error \
        "${BASE_URL}/icons/${icon}.svg" \
        --output "${ICON_DESTINATION}/${icon}.svg"
done

curl \
    --fail \
    --location \
    --silent \
    --show-error \
    "${BASE_URL}/LICENSE" \
    --output "${DESTINATION}/LICENSE"

echo "Lucide icons installed in ${DESTINATION}"