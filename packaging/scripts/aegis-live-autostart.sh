#!/bin/bash
# Openbox autostart — GUI Live only (not CLI / Forensic / Air-Gap)
set -euo pipefail

grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && exit 0

MODE="$(cat /run/aegis/mode 2>/dev/null || echo live)"
case "${MODE}" in
    forensic|airgap|cli) exit 0 ;;
esac

export DISPLAY="${DISPLAY:-:0}"

# 가벼운 패널/배경 (openbox만 있으면 빈 화면)
if command -v nitrogen >/dev/null 2>&1; then
    nitrogen --set-color '#0d1117' 2>/dev/null || true
fi

sleep 0.5

if [[ ! -f /var/lib/aegis/.installed ]] && command -v aegis-installer >/dev/null 2>&1; then
    if ! pgrep -x aegis-installer >/dev/null 2>&1; then
        aegis-installer &
    fi
fi
