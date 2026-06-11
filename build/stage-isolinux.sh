#!/usr/bin/env bash
# Stage ISOLINUX/SYSLINUX files for hybrid ISO (BIOS boot).
# Ubuntu 24.04+ splits isolinux.bin (/usr/lib/ISOLINUX) and ldlinux.c32
# (/usr/lib/syslinux/modules/bios) — both are required since Syslinux 5+.
stage_isolinux_files() {
    local iso_stage="$1"
    local dest="${iso_stage}/isolinux"

    mkdir -p "${dest}"

    local isolinux_dirs=(
        "/usr/lib/ISOLINUX"
        "/usr/lib/syslinux/bios"
        "/usr/share/syslinux"
    )
    local module_dirs=(
        "/usr/lib/syslinux/modules/bios"
        "/usr/lib/syslinux/bios"
        "/usr/share/syslinux/modules/bios"
        "/usr/lib/ISOLINUX"
    )

    local d f
    for d in "${isolinux_dirs[@]}"; do
        [[ -d "${d}" ]] || continue
        for f in isohdpfx.bin isolinux.bin; do
            if [[ -f "${d}/${f}" && ! -f "${dest}/${f}" ]]; then
                cp "${d}/${f}" "${dest}/"
            fi
        done
    done

    for d in "${module_dirs[@]}"; do
        [[ -d "${d}" ]] || continue
        for f in ldlinux.c32 libcom32.c32 libutil.c32 vesamenu.c32; do
            if [[ -f "${d}/${f}" && ! -f "${dest}/${f}" ]]; then
                cp "${d}/${f}" "${dest}/"
            fi
        done
    done

    if [[ ! -f "${dest}/isolinux.bin" ]]; then
        return 1
    fi
    if [[ ! -f "${dest}/ldlinux.c32" ]]; then
        return 2
    fi
    return 0
}
