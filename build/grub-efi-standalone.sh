#!/usr/bin/env bash
# GRUB EFI — Ubuntu-style: embed cfg + theme + DejaVu fonts in BOOTX64.EFI

filter_grub_efi_modules() {
    local grub_dir="${1:-/usr/lib/grub/x86_64-efi}"
    local -a want=("${@:2}")
    local -a filtered=()
    local mod

    for mod in "${want[@]}"; do
        if [[ -f "${grub_dir}/${mod}.mod" ]]; then
            filtered+=("$mod")
        fi
    done

    printf '%s' "${filtered[*]}"
}

grub_efi_build_embed_args() {
    local iso_stage="$1"
    local grub_cfg_src="$2"
    local -a embed=("boot/grub/grub.cfg=${grub_cfg_src}")

    local f dir
    for dir in "${iso_stage}/boot/grub/fonts" /usr/share/grub; do
        [[ -d "${dir}" ]] || continue
        for f in unicode.pf2; do
            if [[ -f "${dir}/${f}" && ! " ${embed[*]} " =~ boot/grub/fonts/${f}= ]]; then
                embed+=("boot/grub/fonts/${f}=${dir}/${f}")
            fi
        done
    done

    printf '%s\0' "${embed[@]}"
}

grub_efi_mkstandalone() {
    local iso_stage="$1"
    local grub_cfg_src="$2"
    local efi_out="$3"

    command -v grub-mkstandalone >/dev/null 2>&1 || return 1
    local grub_dir="/usr/lib/grub/x86_64-efi"
    [[ -d "$grub_dir" ]] || return 1
    [[ -f "$grub_cfg_src" ]] || return 1

    local -a want_modules=(
        all_video boot bufio configfile echo efi_gop ext2 fat fwsetup gfxterm
        iso9660 linux loadenv loopback normal part_gpt part_msdos
        reboot search search_fs_file serial terminal test
    )
    local modules logfile
    modules="$(filter_grub_efi_modules "${grub_dir}" "${want_modules[@]}")"
    [[ -n "$modules" ]] || return 1

    local -a embed=()
    local item
    while IFS= read -r -d '' item; do
        embed+=("$item")
    done < <(grub_efi_build_embed_args "$iso_stage" "$grub_cfg_src")

    logfile="$(mktemp /tmp/aegis-grub-mkstandalone.XXXXXX.log)"
    if ! grub-mkstandalone \
        --format=x86_64-efi \
        --output="${efi_out}" \
        --locales="" \
        --fonts=unicode \
        --modules="${modules}" \
        "${embed[@]}" >"${logfile}" 2>&1; then
        echo "grub-mkstandalone failed:" >&2
        tail -30 "${logfile}" >&2 || true
        rm -f "${logfile}"
        return 1
    fi
    rm -f "${logfile}"
    [[ -f "$efi_out" ]] && return 0
    return 1
}

create_grub_efi_standalone() {
    local iso_stage="$1"
    local grub_cfg_src="$2"
    local efi_img="$3"

    local efi_tmp
    efi_tmp="$(mktemp /tmp/aegis-grubx64.XXXXXX.efi)"
    if ! grub_efi_mkstandalone "$iso_stage" "$grub_cfg_src" "$efi_tmp"; then
        rm -f "${efi_tmp}"
        return 1
    fi

    rm -f "${efi_img}"
    dd if=/dev/zero bs=1M count=10 of="${efi_img}" 2>/dev/null
    mkfs.vfat "${efi_img}" 2>/dev/null || { rm -f "${efi_tmp}"; return 1; }
    mmd -i "${efi_img}" ::/EFI ::/EFI/BOOT 2>/dev/null || true
    mcopy -i "${efi_img}" "${efi_tmp}" ::/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true
    rm -f "${efi_tmp}"
    [[ -f "${efi_img}" ]] && return 0
    return 1
}

create_grub_efi_boot_file() {
    local iso_stage="$1"
    local grub_cfg_src="$2"
    local efi_out="$3"
    grub_efi_mkstandalone "$iso_stage" "$grub_cfg_src" "$efi_out"
}

ensure_grub_pf2_font() {
    local pf2_name="$1"
    local dest="$2"

    [[ -f "$dest" ]] && return 0

    local found
    found=$(find /usr/share/grub /usr/lib/grub -name "${pf2_name}" 2>/dev/null | head -1)
    if [[ -n "$found" ]]; then
        cp "$found" "$dest"
        return 0
    fi

    command -v grub-mkfont >/dev/null 2>&1 || return 1

    case "${pf2_name}" in
        DejaVuSans-Regular-16.pf2)
            if [[ -f /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf ]]; then
                grub-mkfont -o "$dest" -s 16 /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
                [[ -f "$dest" ]] && return 0
            fi
            ;;
        DejaVuSans-Bold-16.pf2)
            if [[ -f /usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf ]]; then
                grub-mkfont -o "$dest" -s 16 /usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf
                [[ -f "$dest" ]] && return 0
            fi
            ;;
        unicode.pf2)
            if [[ -f /usr/share/grub/unicode.pf2 ]]; then
                cp /usr/share/grub/unicode.pf2 "$dest"
                return 0
            fi
            ;;
    esac
    return 1
}

stage_grub_ubuntu_boot() {
    local project_root="$1"
    local iso_stage="$2"

    mkdir -p "${iso_stage}/boot/grub/fonts"
    cp "${project_root}/build/grub/grub.cfg" "${iso_stage}/boot/grub/grub.cfg"

    rm -rf "${iso_stage}/boot/grub/themes" \
           "${iso_stage}/boot/grub/aegis-bg.png" 2>/dev/null || true

    if ! ensure_grub_pf2_font "unicode.pf2" "${iso_stage}/boot/grub/fonts/unicode.pf2"; then
        echo "stage_grub: unicode.pf2 missing — apt install grub-common" >&2
        return 1
    fi
    return 0
}
