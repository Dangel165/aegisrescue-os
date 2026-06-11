#!/bin/bash
# Openbox autostart — launch installer once when X session is ready (GUI Live only)
grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && exit 0

MODE="$(cat /run/aegis/mode 2>/dev/null || echo live)"
case "${MODE}" in
    forensic|airgap|cli) exit 0 ;;
esac

export DISPLAY="${DISPLAY:-:0}"
LOG=/tmp/aegis-autostart.log
INSTALLER=/opt/aegis/bin/aegis-installer-gui.sh
SPLASH=/usr/share/aegis/aegis-boot-splash.png
GUARD=/run/aegis/installer-launched

mkdir -p /run/aegis 2>/dev/null || true
if [[ -f "${GUARD}" ]]; then
    exit 0
fi

{
    echo "=== aegis-live-autostart $(date -Is) DISPLAY=${DISPLAY} ==="
} >>"${LOG}" 2>&1

if [[ ! -S /tmp/.X11-unix/X0 ]]; then
    echo "no X socket — skip installer" >>"${LOG}"
    exit 0
fi

if command -v nitrogen >/dev/null 2>&1; then
    if [[ -f "${SPLASH}" ]]; then
        nitrogen --set-centered "${SPLASH}" >>"${LOG}" 2>&1 || \
        nitrogen --set-scaled "${SPLASH}" >>"${LOG}" 2>&1 || \
        nitrogen --set-color '#000000' >>"${LOG}" 2>&1 || true
    else
        nitrogen --set-color '#000000' >>"${LOG}" 2>&1 || true
    fi
fi

sleep 2

if [[ -f /var/lib/aegis/.installed ]]; then
    echo "already installed — skip installer" >>"${LOG}"
    touch "${GUARD}" 2>/dev/null || true
    exit 0
fi

if pgrep -x aegis-installer >/dev/null 2>&1; then
    echo "installer already running" >>"${LOG}"
    touch "${GUARD}" 2>/dev/null || true
    exit 0
fi

if [[ ! -x "${INSTALLER}" ]]; then
    echo "missing: ${INSTALLER}" >>"${LOG}"
    touch "${GUARD}" 2>/dev/null || true
    exit 0
fi

touch "${GUARD}" 2>/dev/null || true
nohup "${INSTALLER}" >>"${LOG}" 2>&1 &
echo "launched aegis-installer pid=$!" >>"${LOG}"
