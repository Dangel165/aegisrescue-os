#!/bin/bash
# X session for Live GUI — Openbox + installer autostart (VirtualBox-friendly)
export DISPLAY="${DISPLAY:-:0}"
xsetroot -solid "#0d1117" 2>/dev/null || true
if command -v nitrogen >/dev/null 2>&1; then
    nitrogen --set-color '#0d1117' 2>/dev/null || true
fi
exec openbox-session
