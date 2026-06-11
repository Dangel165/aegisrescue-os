#!/bin/bash
# profile.d hook — GUI Live: start X on tty1 (no LightDM, works in VirtualBox)
grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && return 0
grep -Eq '(^| )aegis\.mode=(forensic|airgap)($| )' /proc/cmdline 2>/dev/null && return 0
[[ -n "${DISPLAY:-}" ]] && return 0
[[ "$(tty 2>/dev/null)" != "/dev/tty1" ]] && return 0
pgrep -x Xorg >/dev/null 2>&1 && return 0
command -v startx >/dev/null 2>&1 || return 0

if command -v plymouth >/dev/null 2>&1; then
    plymouth quit 2>/dev/null || true
    sleep 1
fi

: > /tmp/aegis-startx.log
if startx -- :0 vt1 -nolisten tcp -noreset >>/tmp/aegis-startx.log 2>&1; then
    exit 0
fi

echo "AegisRescue: startx failed — see /tmp/aegis-startx.log and /var/log/Xorg.0.log" >&2
return 0
