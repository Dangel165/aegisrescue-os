#!/usr/bin/env bash
# Render docs/assets/aegisrescue-logo.svg → assets/aegisrescue-boot-logo-black.png
set -euo pipefail

src="${1:?SVG source path required}"
dst="${2:?PNG destination path required}"

mkdir -p "$(dirname "$dst")"

if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 1920 "$src" -o "$dst"
elif command -v inkscape >/dev/null 2>&1; then
    inkscape "$src" -w 1920 -o "$dst"
else
    echo "render-boot-logo: install librsvg2-bin (rsvg-convert) or inkscape" >&2
    exit 1
fi
