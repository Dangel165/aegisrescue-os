#!/bin/bash
# Fallback if LightDM did not bring up X within 20s (VirtualBox / VM edge cases)
set -u

LOG=/tmp/aegis-gui-fallback.log
MARK=/run/aegis/gui-fallback-done

grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null && exit 0

mkdir -p /run/aegis
touch "${MARK}" 2>/dev/null || true

{
    echo "=== aegis-gui-fallback $(date -Is) ==="
} >>"${LOG}"

for _ in $(seq 1 20); do
    [[ -S /tmp/.X11-unix/X0 ]] && { echo "X0 up — nothing to do" >>"${LOG}"; exit 0; }
    sleep 1
done

echo "X0 missing after 20s — try lightdm restart" >>"${LOG}"
systemctl restart lightdm.service >>"${LOG}" 2>&1 || true

sleep 5
if [[ -S /tmp/.X11-unix/X0 ]]; then
    echo "lightdm restart ok" >>"${LOG}"
    exit 0
fi

echo "lightdm failed — startx on tty1 as aegis" >>"${LOG}"
if command -v chvt >/dev/null 2>&1; then
    chvt 1 2>>"${LOG}" || true
fi

if [[ -x /opt/aegis/bin/aegis-startx.sh ]]; then
    su - aegis -c '/opt/aegis/bin/aegis-startx.sh' >>"${LOG}" 2>&1 || true
fi

exit 0
