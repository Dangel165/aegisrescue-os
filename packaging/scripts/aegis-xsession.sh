#!/bin/bash
# X session for Live GUI — Openbox + installer autostart
export DISPLAY="${DISPLAY:-:0}"
SPLASH="/usr/share/aegis/aegis-boot-splash.png"

xsetroot -solid "#000000" 2>/dev/null || true
if command -v nitrogen >/dev/null 2>&1; then
    if [[ -f "${SPLASH}" ]]; then
        nitrogen --set-centered "${SPLASH}" 2>/dev/null || \
        nitrogen --set-scaled "${SPLASH}" 2>/dev/null || \
        nitrogen --set-color '#000000' 2>/dev/null || true
    else
        nitrogen --set-color '#000000' 2>/dev/null || true
    fi
fi
exec openbox-session
