#!/usr/bin/env bash
# Stage boot logos for GRUB, ISOLINUX, Plymouth, and desktop wallpaper.

# Logo on black canvas — pct = logo width as % of canvas width (e.g. 30)
prepare_centered_boot_logo_png() {
    local src_png="$1"
    local dst_png="$2"
    local canvas_w="$3"
    local canvas_h="$4"
    local pct="${5:-30}"
    local src_svg="${6:-}"

    mkdir -p "$(dirname "$dst_png")"

    if command -v convert >/dev/null 2>&1; then
        convert -size "${canvas_w}x${canvas_h}" xc:black \
            \( "$src_png" -resize "${pct}%" \) \
            -gravity center -composite PNG:"$dst_png" 2>/dev/null && return 0
    fi

    if [[ -f "$src_svg" ]] && command -v rsvg-convert >/dev/null 2>&1; then
        local logo_w=$(( canvas_w * pct / 100 ))
        local logo_h=$(( canvas_h * pct / 100 ))
        rsvg-convert -w "${logo_w}" -h "${logo_h}" "$src_svg" -o "$dst_png" 2>/dev/null && return 0
    fi

    local logo_w=$(( canvas_w * pct / 100 ))
    local logo_h=$(( canvas_h * pct / 100 ))
    if command -v convert >/dev/null 2>&1; then
        convert "$src_png" -resize "${logo_w}x${logo_h}!" -depth 24 PNG:"$dst_png" 2>/dev/null && return 0
    fi
    cp "$src_png" "$dst_png"
}

prepare_boot_logo_png() {
    prepare_centered_boot_logo_png "$1" "$2" "$3" "$4" 30 "${5:-}"
}

prepare_menu_splash_png() {
    prepare_centered_boot_logo_png "$1" "$2" 800 450 30 "${3:-}"
}

stage_iso_boot_logos() {
    local project_root="$1"
    local iso_stage="$2"

    # shellcheck source=resolve-boot-logo.sh
    source "${project_root}/build/resolve-boot-logo.sh"

    local logo_src=""
    logo_src="$(resolve_boot_logo "${project_root}" || true)"
    [[ -n "$logo_src" ]] || return 1

    local logo_svg="${project_root}/docs/assets/aegisrescue-logo.svg"
    mkdir -p "${iso_stage}/boot/grub/themes/aegis" "${iso_stage}/isolinux"

    prepare_centered_boot_logo_png "$logo_src" "${iso_stage}/boot/grub/aegis-bg.png" 800 450 30 "$logo_svg"
    prepare_centered_boot_logo_png "$logo_src" "${iso_stage}/boot/grub/themes/aegis/aegis-bg.png" 800 450 30 "$logo_svg"
    prepare_menu_splash_png "$logo_src" "${iso_stage}/isolinux/splash.png" "$logo_svg"
    return 0
}

install_chroot_boot_logos() {
    local project_root="$1"
    local chroot="$2"

    # shellcheck source=resolve-boot-logo.sh
    source "${project_root}/build/resolve-boot-logo.sh"

    local logo_src=""
    logo_src="$(resolve_boot_logo "${project_root}" || true)"
    [[ -n "$logo_src" ]] || return 1

    local logo_svg="${project_root}/docs/assets/aegisrescue-logo.svg"
    mkdir -p "${chroot}/usr/share/plymouth/themes/aegisrescue"
    cp "${project_root}/build/plymouth/aegisrescue.plymouth" \
       "${chroot}/usr/share/plymouth/themes/aegisrescue/" 2>/dev/null || true
    cp "${project_root}/build/plymouth/aegisrescue.script" \
       "${chroot}/usr/share/plymouth/themes/aegisrescue/" 2>/dev/null || true
    cp "$logo_src" "${chroot}/usr/share/aegis/aegisrescue-boot-logo.png"
    prepare_centered_boot_logo_png "$logo_src" \
        "${chroot}/usr/share/plymouth/themes/aegisrescue/aegis-bg.png" 640 360 30 "$logo_svg"

    mkdir -p "${chroot}/etc/plymouth"
    cat > "${chroot}/etc/plymouth/plymouthd.conf" <<'EOF'
[Daemon]
Theme=aegisrescue
ShowDelay=0
EOF

    chroot "${chroot}" update-alternatives --install \
        /usr/share/plymouth/themes/default.plymouth default.plymouth \
        /usr/share/plymouth/themes/aegisrescue/aegisrescue.plymouth 100 2>/dev/null || true
    chroot "${chroot}" update-alternatives --set default.plymouth \
        /usr/share/plymouth/themes/aegisrescue/aegisrescue.plymouth 2>/dev/null || true
    chroot "${chroot}" plymouth-set-default-theme aegisrescue 2>/dev/null || true
    return 0
}
