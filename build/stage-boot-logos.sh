#!/usr/bin/env bash
# Stage boot logos for GRUB, ISOLINUX, Plymouth, and desktop wallpaper.

# Full-screen black canvas + centered logo (~Windows boot splash proportions)
AEGIS_BOOT_CANVAS_W="${AEGIS_BOOT_CANVAS_W:-1920}"
AEGIS_BOOT_CANVAS_H="${AEGIS_BOOT_CANVAS_H:-1080}"
AEGIS_BOOT_LOGO_PCT="${AEGIS_BOOT_LOGO_PCT:-8}"
# Plymouth displays PNG 1:1 — smaller canvas keeps logo modest on large displays
AEGIS_PLYMOUTH_CANVAS_W="${AEGIS_PLYMOUTH_CANVAS_W:-960}"
AEGIS_PLYMOUTH_CANVAS_H="${AEGIS_PLYMOUTH_CANVAS_H:-540}"

resolve_boot_mark_svg() {
    local project_root="$1"
    local boot_mark="${project_root}/docs/assets/aegisrescue-boot-mark.svg"
    local full_logo="${project_root}/docs/assets/aegisrescue-logo.svg"
    if [[ -f "$boot_mark" ]]; then
        printf '%s\n' "$boot_mark"
    elif [[ -f "$full_logo" ]]; then
        printf '%s\n' "$full_logo"
    fi
}

# Logo on black canvas — pct = logo width as % of canvas width (e.g. 15)
prepare_centered_boot_logo_png() {
    local src_png="$1"
    local dst_png="$2"
    local canvas_w="$3"
    local canvas_h="$4"
    local pct="${5:-${AEGIS_BOOT_LOGO_PCT}}"
    local src_svg="${6:-}"
    local logo_w=$(( canvas_w * pct / 100 ))

    mkdir -p "$(dirname "$dst_png")"

    if [[ -f "$src_svg" ]] && command -v rsvg-convert >/dev/null 2>&1; then
        local tmp_logo
        tmp_logo="$(mktemp /tmp/aegis-logo.XXXXXX.png)"
        rsvg-convert -w "${logo_w}" "$src_svg" -o "${tmp_logo}" 2>/dev/null || true
        if [[ -s "$tmp_logo" ]] && command -v convert >/dev/null 2>&1; then
            convert -size "${canvas_w}x${canvas_h}" xc:black \
                "$tmp_logo" -gravity center -composite PNG:"$dst_png" 2>/dev/null && {
                rm -f "$tmp_logo"
                return 0
            }
        fi
        rm -f "$tmp_logo"
    fi

    if command -v convert >/dev/null 2>&1; then
        convert -size "${canvas_w}x${canvas_h}" xc:black \
            \( "$src_png" -resize "${logo_w}x" \) \
            -gravity center -composite PNG:"$dst_png" 2>/dev/null && return 0
    fi

    if [[ -f "$src_svg" ]] && command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "${logo_w}" -b black "$src_svg" -o "$dst_png" 2>/dev/null && return 0
    fi

    echo "prepare_centered_boot_logo_png: install imagemagick + librsvg2-bin" >&2
    if command -v convert >/dev/null 2>&1; then
        convert -size "${canvas_w}x${canvas_h}" xc:black PNG:"$dst_png" 2>/dev/null && return 0
    fi
    return 1
}

prepare_boot_logo_png() {
    prepare_centered_boot_logo_png "$1" "$2" "$3" "$4" "${AEGIS_BOOT_LOGO_PCT}" "${5:-}"
}

prepare_menu_splash_png() {
    prepare_centered_boot_logo_png "$1" "$2" 800 450 "${AEGIS_BOOT_LOGO_PCT}" "${3:-}"
}

stage_iso_boot_logos() {
    local project_root="$1"
    local iso_stage="$2"

    # shellcheck source=resolve-boot-logo.sh
    source "${project_root}/build/resolve-boot-logo.sh"

    local logo_src=""
    logo_src="$(resolve_boot_logo "${project_root}" || true)"
    [[ -n "$logo_src" ]] || return 1

    local logo_svg
    logo_svg="$(resolve_boot_mark_svg "${project_root}")"
    mkdir -p "${iso_stage}/boot/grub/themes/aegis" "${iso_stage}/isolinux"

    prepare_centered_boot_logo_png "$logo_src" "${iso_stage}/boot/grub/aegis-bg.png" \
        "${AEGIS_BOOT_CANVAS_W}" "${AEGIS_BOOT_CANVAS_H}" "${AEGIS_BOOT_LOGO_PCT}" "$logo_svg"
    prepare_centered_boot_logo_png "$logo_src" "${iso_stage}/boot/grub/themes/aegis/aegis-bg.png" \
        "${AEGIS_BOOT_CANVAS_W}" "${AEGIS_BOOT_CANVAS_H}" "${AEGIS_BOOT_LOGO_PCT}" "$logo_svg"
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

    local logo_svg
    logo_svg="$(resolve_boot_mark_svg "${project_root}")"
    mkdir -p "${chroot}/usr/share/plymouth/themes/aegisrescue"
    cp "${project_root}/build/plymouth/aegisrescue.plymouth" \
       "${chroot}/usr/share/plymouth/themes/aegisrescue/" 2>/dev/null || true
    cp "${project_root}/build/plymouth/aegisrescue.script" \
       "${chroot}/usr/share/plymouth/themes/aegisrescue/" 2>/dev/null || true
    cp "$logo_src" "${chroot}/usr/share/aegis/aegisrescue-boot-logo.png"
    # Plymouth: smaller canvas (1:1 pixel display) + shield-only mark
    if ! prepare_centered_boot_logo_png "$logo_src" \
        "${chroot}/usr/share/plymouth/themes/aegisrescue/aegis-bg.png" \
        "${AEGIS_PLYMOUTH_CANVAS_W}" "${AEGIS_PLYMOUTH_CANVAS_H}" "${AEGIS_BOOT_LOGO_PCT}" "$logo_svg"; then
        echo "warn: Plymouth aegis-bg.png compositing failed — check imagemagick" >&2
    fi
    if ! prepare_centered_boot_logo_png "$logo_src" \
        "${chroot}/usr/share/aegis/aegis-boot-splash.png" \
        "${AEGIS_BOOT_CANVAS_W}" "${AEGIS_BOOT_CANVAS_H}" "${AEGIS_BOOT_LOGO_PCT}" "$logo_svg"; then
        cp "$logo_src" "${chroot}/usr/share/aegis/aegis-boot-splash.png" 2>/dev/null || true
    fi

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
