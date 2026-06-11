#!/usr/bin/env bash
# Resolve boot logo PNG for GRUB / ISOLINUX / Plymouth / desktop wallpaper.

resolve_boot_logo() {
    local project_root="$1"
    local logo_png="${project_root}/assets/aegisrescue-boot-logo-black.png"
    local logo_svg="${project_root}/docs/assets/aegisrescue-logo.svg"
    local candidate=""

    if [[ -n "${AEGIS_BOOT_LOGO:-}" && -f "${AEGIS_BOOT_LOGO}" ]]; then
        printf '%s\n' "${AEGIS_BOOT_LOGO}"
        return 0
    fi

    if [[ -f "$logo_png" ]]; then
        printf '%s\n' "$logo_png"
        return 0
    fi

    if [[ -f "$logo_svg" && -x "${project_root}/build/render-boot-logo.sh" ]]; then
        if "${project_root}/build/render-boot-logo.sh" "$logo_svg" "$logo_png"; then
            printf '%s\n' "$logo_png"
            return 0
        fi
    fi

    return 1
}
