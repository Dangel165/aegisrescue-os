#!/bin/bash
# X session for Live GUI — Openbox + installer autostart (VirtualBox-friendly)
export DISPLAY="${DISPLAY:-:0}"
LOGO="/usr/share/aegis/aegisrescue-boot-logo.png"

xsetroot -solid "#0d1117" 2>/dev/null || true
if command -v nitrogen >/dev/null 2>&1; then
    if [[ -f "${LOGO}" ]]; then
        nitrogen --set-scaled "${LOGO}" 2>/dev/null || nitrogen --set-color '#0d1117' 2>/dev/null || true
    else
        nitrogen --set-color '#0d1117' 2>/dev/null || true
    fi
fi
exec openbox-session
