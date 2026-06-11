#!/usr/bin/env bash
# Build /usr/share/aegis/vm/* artifacts for L2 sandbox (Firecracker + QEMU).
# Uses a tiny hand-built rootfs — no debootstrap (avoids /tmp disk exhaustion in WSL).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VM_DIR="${ROOT}/data/vm"
mkdir -p "${VM_DIR}"

log() { echo "[build-vm] $*"; }
need() { command -v "$1" >/dev/null 2>&1; }

ROOTFS="${VM_DIR}/rootfs.ext4"
QCOW="${VM_DIR}/analysis.qcow2"
KERNEL="${VM_DIR}/vmlinux.bin"
STAGING="${VM_DIR}/.rootfs-staging"
MNT="${VM_DIR}/.rootfs-mnt"

cleanup() {
  if mountpoint -q "${MNT}" 2>/dev/null; then
    umount "${MNT}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

build_rootfs() {
  if [[ -f "${ROOTFS}" ]]; then
    local size=0
    size=$(stat -c%s "${ROOTFS}" 2>/dev/null || stat -f%z "${ROOTFS}" 2>/dev/null || echo 0)
    # Drop oversized images from older debootstrap-based builds
    if [[ "${size}" -gt 33554432 ]]; then
      log "removing oversized rootfs (${size} bytes) from previous build"
      rm -f "${ROOTFS}"
    elif [[ "${ROOTFS}" -nt "${BASH_SOURCE[0]}" ]]; then
      log "rootfs.ext4 exists — skip"
      return 0
    fi
  fi

  rm -rf "${STAGING}" "${MNT}"
  mkdir -p "${STAGING}"/{bin,sbin,etc,proc,sys,dev,tmp,usr/share/aegis}
  mkdir -p "${MNT}"

  cat > "${STAGING}/sbin/init" <<'INIT'
#!/bin/sh
mount -t proc proc /proc 2>/dev/null || true
mount -t sysfs sys /sys 2>/dev/null || true
echo "AegisRescue analysis VM ready"
sleep 3600
poweroff -f
INIT
  chmod +x "${STAGING}/sbin/init"
  echo "aegis-vm" > "${STAGING}/etc/hostname"
  echo "minimal-aegis-rootfs" > "${STAGING}/usr/share/aegis/vm-marker"

  log "creating rootfs.ext4 (16M minimal)..."
  rm -f "${ROOTFS}"
  truncate -s 16M "${ROOTFS}"
  mkfs.ext4 -F -L aegis-rootfs "${ROOTFS}" >/dev/null

  if need mount && need rsync; then
    mount -o loop "${ROOTFS}" "${MNT}"
    rsync -a "${STAGING}/" "${MNT}/"
    umount "${MNT}"
    log "populated minimal rootfs"
  else
    log "warn: loop mount unavailable — empty ext4 (run on Linux with root/sudo)"
  fi

  rm -rf "${STAGING}" "${MNT}"
  trap - EXIT
}

build_qcow() {
  if [[ -f "${QCOW}" ]]; then
    log "analysis.qcow2 exists — skip"
    return 0
  fi
  if need qemu-img; then
    qemu-img create -f qcow2 "${QCOW}" 256M >/dev/null
    log "created ${QCOW}"
  else
    log "warn: qemu-img missing — skipping analysis.qcow2"
  fi
}

build_kernel() {
  if [[ -f "${KERNEL}" && "${KERNEL}" -nt "${BASH_SOURCE[0]}" ]]; then
    log "vmlinux.bin exists — skip"
    return 0
  fi

  local candidate=""
  for k in /boot/vmlinuz-* /boot/vmlinuz; do
    [[ -f "${k}" ]] && candidate="${k}" && break
  done

  if [[ -n "${candidate}" ]]; then
    if need extract-vmlinux; then
      extract-vmlinux "${candidate}" > "${KERNEL}" 2>/dev/null && {
        log "extracted vmlinux from ${candidate}"
        return 0
      }
    fi
    cp "${candidate}" "${KERNEL}"
    log "copied kernel from ${candidate} (Firecracker may need extract-vmlinux)"
    return 0
  fi

  log "warn: no host kernel — writing placeholder vmlinux.bin"
  printf 'AEGIS_VM_KERNEL_PLACEHOLDER\nBuild on Linux with /boot/vmlinuz or set AEGIS_VM_KERNEL\n' > "${KERNEL}"
}

build_kernel
build_rootfs
build_qcow

cat > "${VM_DIR}/README.txt" <<EOF
AegisRescue L2 sandbox VM artifacts
- vmlinux.bin   Firecracker kernel (extract-vmlinux from host vmlinuz recommended)
- rootfs.ext4   Minimal 16M analysis rootfs (busybox-less stub init)
- analysis.qcow2 QEMU fallback disk image
Installed to /usr/share/aegis/vm/ by wsl-build.sh
EOF

log "done -> ${VM_DIR}"
