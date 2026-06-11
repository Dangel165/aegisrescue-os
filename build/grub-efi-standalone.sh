#!/usr/bin/env bash
# Build GRUB EFI standalone image with embedded cfg, font, theme, and boot logo.

create_grub_efi_standalone() {
    local iso_stage="$1"
    local grub_cfg_src="$2"
    local efi_img="$3"

    command -v grub-mkstandalone >/dev/null 2>&1 || return 1

    local efi_tmp
    efi_tmp="$(mktemp /tmp/aegis-grubx64.XXXXXX.efi)"
    local -a embed=(
        "boot/grub/grub.cfg=${grub_cfg_src}"
    )

    local font="${iso_stage}/boot/grub/fonts/unicode.pf2"
    [[ -f "$font" ]] && embed+=("boot/grub/fonts/unicode.pf2=${font}")

    local bg="${iso_stage}/boot/grub/aegis-bg.png"
    [[ -f "$bg" ]] && embed+=("boot/grub/aegis-bg.png=${bg}")

    local theme_txt="${iso_stage}/boot/grub/themes/aegis/theme.txt"
    [[ -f "$theme_txt" ]] && embed+=("boot/grub/themes/aegis/theme.txt=${theme_txt}")

    local theme_bg="${iso_stage}/boot/grub/themes/aegis/aegis-bg.png"
    [[ -f "$theme_bg" ]] && embed+=("boot/grub/themes/aegis/aegis-bg.png=${theme_bg}")

    if ! grub-mkstandalone \
        --format=x86_64-efi \
        --output="${efi_tmp}" \
        --locales="" \
        --themes="" \
        "${embed[@]}" 2>/dev/null; then
        rm -f "${efi_tmp}"
        return 1
    fi

    dd if=/dev/zero bs=1M count=10 of="${efi_img}" 2>/dev/null
    mkfs.vfat "${efi_img}" 2>/dev/null || { rm -f "${efi_tmp}"; return 1; }
    mmd -i "${efi_img}" ::/EFI ::/EFI/BOOT 2>/dev/null || true
    mcopy -i "${efi_img}" "${efi_tmp}" ::/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true
    rm -f "${efi_tmp}"
    [[ -f "${efi_img}" ]] && return 0
    return 1
}
