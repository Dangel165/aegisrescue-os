#!/bin/bash
# Launch installer with root privileges on Live GUI (disk install needs root).
grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && exec /opt/aegis/bin/aegis-installer "$@"

INSTALLER=/opt/aegis/bin/aegis-installer
export DISPLAY="${DISPLAY:-:0}"

if [[ $EUID -eq 0 ]]; then
    exec "${INSTALLER}" "$@"
fi

if sudo -n true 2>/dev/null; then
    exec sudo -E env DISPLAY="${DISPLAY}" XAUTHORITY="${XAUTHORITY:-$HOME/.Xauthority}" \
        "${INSTALLER}" "$@"
fi

exec "${INSTALLER}" "$@"
