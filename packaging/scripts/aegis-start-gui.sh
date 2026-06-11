#!/bin/bash
# profile.d hook — GUI Live: start X on tty1 once per boot (no LightDM)
grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && return 0
grep -Eq '(^| )aegis\.mode=(forensic|airgap)($| )' /proc/cmdline 2>/dev/null && return 0
[[ -n "${DISPLAY:-}" ]] && return 0
[[ "$(tty 2>/dev/null)" != "/dev/tty1" ]] && return 0
pgrep -x Xorg >/dev/null 2>&1 && return 0
command -v startx >/dev/null 2>&1 || return 0

# Prevent getty/login ↔ startx flicker loop (only one attempt per boot)
_aegis_guard="/run/aegis/gui-attempted"
mkdir -p /run/aegis 2>/dev/null || true
if [[ -f "${_aegis_guard}" ]]; then
    return 0
fi
touch "${_aegis_guard}" 2>/dev/null || true

if command -v plymouth >/dev/null 2>&1; then
    plymouth quit 2>/dev/null || true
    sleep 0.5
fi

: > /tmp/aegis-startx.log
if /opt/aegis/bin/aegis-startx.sh; then
    return 0
fi

echo "AegisRescue: startx failed — see /tmp/aegis-startx.log and /var/log/Xorg.0.log" >&2
return 0
