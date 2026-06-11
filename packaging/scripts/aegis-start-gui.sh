#!/bin/bash
# profile.d hook — GUI Live: start X on tty1 (no LightDM, works in VirtualBox)
grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && return 0
grep -Eq '(^| )aegis\.mode=(forensic|airgap)($| )' /proc/cmdline 2>/dev/null && return 0
[[ -n "${DISPLAY:-}" ]] && return 0
[[ "$(tty 2>/dev/null)" != "/dev/tty1" ]] && return 0

AEGIS_GUI_STAMP="${HOME}/.cache/aegis/.gui-started"
[[ -f "${AEGIS_GUI_STAMP}" ]] && return 0
command -v startx >/dev/null 2>&1 || return 0

mkdir -p "${HOME}/.cache/aegis"
touch "${AEGIS_GUI_STAMP}"

# Log failures: cat /tmp/aegis-startx.log on tty2
exec startx /opt/aegis/bin/aegis-xsession -- vt1 -- \
    -nolisten tcp -noreset +extension GLX +extension RANDR +extension RENDER \
    2>/tmp/aegis-startx.log
