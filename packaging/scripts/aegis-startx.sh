#!/bin/bash
# Start X on the active console — tries auto-detected drivers (VirtualBox vmwgfx/modesetting).
set -u

log=/tmp/aegis-startx.log
xorg_log=/var/log/Xorg.0.log
_vt="$(fgconsole 2>/dev/null || echo 1)"

mkdir -p /tmp/.X11-unix
chmod 1777 /tmp/.X11-unix 2>/dev/null || true

uid="$(id -u)"
export HOSTNAME=localhost
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/${uid}}"
mkdir -p "${XDG_RUNTIME_DIR}"
chmod 700 "${XDG_RUNTIME_DIR}" 2>/dev/null || true

{
    echo "=== aegis-startx $(date -Is) vt=${_vt} ==="
    echo "cmdline: $(tr '\n' ' ' </proc/cmdline)"
    echo "groups: $(id)"
    lsmod 2>/dev/null | grep -E 'vmwgfx|drm' || true
} >>"${log}"

run_startx() {
    local label="$1"
    shift
    echo "--- try: ${label} ---" >>"${log}"
    if startx -- :0 -nolisten tcp -noreset vt"${_vt}" "$@" >>"${log}" 2>&1; then
        echo "ok: ${label}" >>"${log}"
        return 0
    fi
    echo "fail: ${label}" >>"${log}"
    [[ -f "${xorg_log}" ]] && tail -40 "${xorg_log}" >>"${log}" 2>/dev/null || true
    return 1
}

# Prefer kernel modesetting (VirtualBox VMSVGA / vmwgfx) — no forced vesa xorg.conf.
if run_startx "auto"; then
    exit 0
fi

# Fallback: explicit fbdev for headless/framebuffer-only VMs.
if [[ -f /dev/fb0 ]]; then
    tmpconf="$(mktemp /tmp/aegis-xorg.XXXXXX.conf)"
    cat >"${tmpconf}" <<EOF
Section "Device"
    Identifier "FbDev"
    Driver "fbdev"
    Option "fbdev" "/dev/fb0"
EndSection
Section "Screen"
    Identifier "Screen0"
    Device "FbDev"
EndSection
EOF
    if run_startx "fbdev" -config "${tmpconf}"; then
        rm -f "${tmpconf}"
        exit 0
    fi
    rm -f "${tmpconf}"
fi

# Last resort: VESA (safe graphics / nomodeset boots).
tmpconf="$(mktemp /tmp/aegis-xorg.XXXXXX.conf)"
cat >"${tmpconf}" <<'EOF'
Section "Device"
    Identifier "Vesa"
    Driver "vesa"
EndSection
Section "Screen"
    Identifier "Screen0"
    Device "Vesa"
EndSection
EOF
if run_startx "vesa" -config "${tmpconf}"; then
    rm -f "${tmpconf}"
    exit 0
fi
rm -f "${tmpconf}"

exit 1
