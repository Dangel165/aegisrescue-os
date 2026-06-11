#!/usr/bin/env bash
# Stage boot logos for GRUB, ISOLINUX, Plymouth, and desktop wallpaper.

# Full-screen black canvas + centered logo (~Windows boot splash proportions)
AEGIS_BOOT_CANVAS_W="${AEGIS_BOOT_CANVAS_W:-1920}"
AEGIS_BOOT_CANVAS_H="${AEGIS_BOOT_CANVAS_H:-1080}"
AEGIS_BOOT_LOGO_PCT="${AEGIS_BOOT_LOGO_PCT:-6}"
# Plymouth: fixed pixel mark (1:1) — avoids full-canvas upscale on some VMs
AEGIS_PLYMOUTH_LOGO_PX="${AEGIS_PLYMOUTH_LOGO_PX:-120}"

ensure_boot_logo_build_deps() {
    if command -v convert >/dev/null 2>&1 && command -v rsvg-convert >/dev/null 2>&1; then
        return 0
    fi
    if [[ ${EUID:-0} -eq 0 ]]; then
        apt-get install -y --no-install-recommends imagemagick librsvg2-bin 2>/dev/null || true
    fi
    if command -v convert >/dev/null 2>&1 && command -v rsvg-convert >/dev/null 2>&1; then
        return 0
    fi
    echo "ensure_boot_logo_build_deps: imagemagick + librsvg2-bin 필요 (sudo bash build/wsl-build.sh --setup)" >&2
    return 1
}

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
            convert -size "${canvas_w}x${canvas_h}" gradient:'#3a3a3a-#0a0a0a' \
                "$tmp_logo" -gravity center -composite PNG:"$dst_png" 2>/dev/null && {
                rm -f "$tmp_logo"
                return 0
            }
        fi
        rm -f "$tmp_logo"
    fi

    if command -v convert >/dev/null 2>&1; then
        convert -size "${canvas_w}x${canvas_h}" gradient:'#3a3a3a-#0a0a0a' \
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

# Shield-only PNG at fixed width for Plymouth (1:1 pixel display)
prepare_plymouth_mark_png() {
    local src_png="$1"
    local dst_png="$2"
    local logo_px="${3:-${AEGIS_PLYMOUTH_LOGO_PX}}"
    local src_svg="${4:-}"

    mkdir -p "$(dirname "$dst_png")"

    if [[ -f "$src_svg" ]] && command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "${logo_px}" "$src_svg" -o "$dst_png" 2>/dev/null && return 0
    fi

    if command -v convert >/dev/null 2>&1; then
        convert "$src_png" -resize "${logo_px}x" PNG:"$dst_png" 2>/dev/null && return 0
    fi

    return 1
}

prepare_boot_logo_png() {
    prepare_centered_boot_logo_png "$1" "$2" "$3" "$4" "${AEGIS_BOOT_LOGO_PCT}" "${5:-}"
}

prepare_menu_splash_png() {
    local src_png="$1"
    local dst_png="$2"
    local src_svg="${3:-}"
    local logo_w=48

    mkdir -p "$(dirname "$dst_png")"

    if command -v convert >/dev/null 2>&1; then
        if [[ -f "$src_svg" ]] && command -v rsvg-convert >/dev/null 2>&1; then
            local tmp_logo
            tmp_logo="$(mktemp /tmp/aegis-isolinux-logo.XXXXXX.png)"
            rsvg-convert -w "${logo_w}" "$src_svg" -o "${tmp_logo}" 2>/dev/null || true
            if [[ -s "$tmp_logo" ]]; then
                convert -size 640x480 gradient:'#404040-#0a0a0a' \
                    "$tmp_logo" -gravity north -geometry +0+48 -composite \
                    PNG:"$dst_png" 2>/dev/null && {
                    rm -f "$tmp_logo"
                    return 0
                }
            fi
            rm -f "$tmp_logo"
        fi
        convert -size 640x480 gradient:'#404040-#0a0a0a' PNG:"$dst_png" 2>/dev/null && return 0
    fi

    prepare_centered_boot_logo_png "$1" "$2" 640 480 6 "${3:-}"
}

stage_iso_boot_logos() {
    local project_root="$1"
    local iso_stage="$2"

    ensure_boot_logo_build_deps || true

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

    ensure_boot_logo_build_deps || true

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
    # Plymouth: small fixed-size mark (Windows-style ~120px), black bg from theme script
    if ! prepare_plymouth_mark_png "$logo_src" \
        "${chroot}/usr/share/plymouth/themes/aegisrescue/logo-mark.png" \
        "${AEGIS_PLYMOUTH_LOGO_PX}" "$logo_svg"; then
        echo "warn: Plymouth logo-mark.png failed — check rsvg-convert/imagemagick" >&2
    fi
    # Legacy fallback for themes expecting aegis-bg.png
    if ! prepare_centered_boot_logo_png "$logo_src" \
        "${chroot}/usr/share/plymouth/themes/aegisrescue/aegis-bg.png" \
        "${AEGIS_BOOT_CANVAS_W}" "${AEGIS_BOOT_CANVAS_H}" "${AEGIS_BOOT_LOGO_PCT}" "$logo_svg"; then
        echo "warn: Plymouth aegis-bg.png compositing failed" >&2
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
