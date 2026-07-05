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
    house
    app-window
    download
    hard-drive
    folder
    folder-open
    folder-plus
    file
    file-text
    file-image
    file-archive
    external-link
    layout-list
    layout-grid
    columns-3
    eye
    volume-2
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

echo "fetching Lucide license"

curl \
    --fail \
    --location \
    --silent \
    --show-error \
    "${BASE_URL}/LICENSE" \
    --output "${DESTINATION}/LICENSE"

echo
echo "Lucide icons installed:"
echo "  ${ICON_DESTINATION}"