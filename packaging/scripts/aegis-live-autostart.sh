#!/bin/bash
# Openbox autostart — launch installer on GUI Live boot (Ubuntu-style)
grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && exit 0

MODE="$(cat /run/aegis/mode 2>/dev/null || echo live)"
case "${MODE}" in
    forensic|airgap|cli) exit 0 ;;
esac

export DISPLAY="${DISPLAY:-:0}"
LOG=/tmp/aegis-autostart.log
INSTALLER=/opt/aegis/bin/aegis-installer
LOGO=/usr/share/aegis/aegisrescue-boot-logo.png

{
    echo "=== aegis-live-autostart $(date -Is) DISPLAY=${DISPLAY} ==="
} >>"${LOG}" 2>&1

if command -v nitrogen >/dev/null 2>&1; then
    if [[ -f "${LOGO}" ]]; then
        nitrogen --set-scaled "${LOGO}" >>"${LOG}" 2>&1 || nitrogen --set-color '#0d1117' >>"${LOG}" 2>&1 || true
    else
        nitrogen --set-color '#0d1117' >>"${LOG}" 2>&1 || true
    fi
fi

# Wait for LightDM session / WebKit display to settle
sleep 2

if [[ -f /var/lib/aegis/.installed ]]; then
    echo "already installed — skip installer" >>"${LOG}"
    exit 0
fi

if pgrep -x aegis-installer >/dev/null 2>&1; then
    echo "installer already running" >>"${LOG}"
    exit 0
fi

if [[ ! -x "${INSTALLER}" ]]; then
    echo "missing: ${INSTALLER}" >>"${LOG}"
    exit 0
fi

nohup "${INSTALLER}" >>"${LOG}" 2>&1 &
echo "launched aegis-installer pid=$!" >>"${LOG}"
