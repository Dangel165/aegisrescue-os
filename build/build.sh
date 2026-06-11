#!/usr/bin/env bash
# =============================================================================
# AegisRescue ISO 빌드 스크립트
# 사용법: sudo ./build/build.sh [옵션]
#
# 옵션:
#   --arch   x86_64|aarch64    대상 아키텍처 (기본: x86_64)
#   --ver    VERSION            버전 문자열   (기본: 0.1.0)
#   --jobs   N                  병렬 빌드 수  (기본: nproc)
#   --skip-debootstrap          debootstrap 건너뜀 (기존 chroot 재사용)
#   --skip-kernel               커널 빌드 건너뜀 (prebuilt 사용)
#   --skip-rust                 Rust 빌드 건너뜀
#   --skip-gui                  GUI 빌드 건너뜀
#   --only-iso                  squashfs + ISO만 생성
#   --out    DIR                출력 디렉터리 (기본: build/out)
#   -h|--help                   도움말
# =============================================================================
set -euo pipefail
IFS=$'\n\t'

# ─── 기본 설정 ────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
ARCH="${ARCH:-x86_64}"
VERSION="${VERSION:-0.1.0}"
RELEASE="${RELEASE:-beta}"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
OUT_DIR="${OUT_DIR:-${PROJECT_ROOT}/build/out}"
CHROOT_DIR="${PROJECT_ROOT}/build/chroot"
ISO_STAGE="${PROJECT_ROOT}/build/iso-stage"
ISO_BASENAME="aegisrescue-${RELEASE}-${VERSION}-${ARCH}"
ISO_NAME="${OUT_DIR}/${ISO_BASENAME}.iso"
ISO_LABEL="AegisRescue Beta ${VERSION}"
KERNEL_VER="${KERNEL_VER:-6.12.0-aegis}"
SKIP_DEBOOTSTRAP=0
SKIP_KERNEL=0
SKIP_RUST=0
SKIP_GUI=0
ONLY_ISO=0

# ─── 색상 출력 ────────────────────────────────────────────────
RED='\033[0;31m'; GRN='\033[0;32m'; YLW='\033[1;33m'
CYN='\033[0;36m'; BLD='\033[1m'; RST='\033[0m'

log()  { echo -e "${CYN}[$(date +%H:%M:%S)]${RST} $*"; }
ok()   { echo -e "${GRN}✓${RST} $*"; }
warn() { echo -e "${YLW}⚠${RST} $*"; }
err()  { echo -e "${RED}✗ FATAL:${RST} $*" >&2; exit 1; }
step() { echo -e "\n${BLD}━━━ $* ━━━${RST}"; }

# ─── 인수 파싱 ────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --arch)             ARCH="$2";    shift 2 ;;
        --ver)              VERSION="$2"; shift 2 ;;
        --jobs)             JOBS="$2";    shift 2 ;;
        --out)              OUT_DIR="$2"; shift 2 ;;
        --skip-debootstrap) SKIP_DEBOOTSTRAP=1; shift ;;
        --skip-kernel)      SKIP_KERNEL=1;      shift ;;
        --skip-rust)        SKIP_RUST=1;        shift ;;
        --skip-gui)         SKIP_GUI=1;         shift ;;
        --only-iso)         ONLY_ISO=1;         shift ;;
        -h|--help)
            grep '^#' "$0" | head -20 | sed 's/^# \?//'
            exit 0 ;;
        *) err "알 수 없는 옵션: $1" ;;
    esac
done

if [[ $ONLY_ISO -eq 1 ]]; then
    SKIP_DEBOOTSTRAP=1; SKIP_KERNEL=1; SKIP_RUST=1; SKIP_GUI=1
fi

# ─── 필수 조건 확인 ───────────────────────────────────────────
check_root() {
    [[ $EUID -eq 0 ]] || err "이 스크립트는 root 권한이 필요합니다 (sudo 사용)"
}

check_deps() {
    step "의존성 확인"
    local required=(debootstrap xorriso mksquashfs grub-mkrescue \
                    xz lz4 zstd rsync curl wget sha256sum)
    local missing=()
    for dep in "${required[@]}"; do
        if ! command -v "$dep" &>/dev/null; then
            missing+=("$dep")
        fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        warn "누락된 도구: ${missing[*]}"
        log "설치 시도 중..."
        apt-get install -y --no-install-recommends \
            debootstrap xorriso squashfs-tools grub-pc-bin \
            grub-efi-amd64-bin grub-efi-amd64-signed \
            xz-utils lz4 zstd rsync curl wget \
            isolinux syslinux-common \
            2>/dev/null || warn "일부 패키지 설치 실패 — 수동 설치 필요"
    fi
    ok "의존성 확인 완료"
}

# ─── STEP 1: debootstrap 기반 시스템 ─────────────────────────
build_base_system() {
    step "STEP 1/6 — 기본 시스템 구성 (debootstrap)"

    if [[ $SKIP_DEBOOTSTRAP -eq 1 && -d "${CHROOT_DIR}" ]]; then
        ok "기존 chroot 재사용: ${CHROOT_DIR}"
        return 0
    fi

    [[ -d "${CHROOT_DIR}" ]] && rm -rf "${CHROOT_DIR}"
    mkdir -p "${CHROOT_DIR}"

    log "Ubuntu 24.04 noble minimal 설치 중 (약 5-10분)..."
    debootstrap \
        --arch="${ARCH/x86_64/amd64}" \
        --variant=minbase \
        --include=systemd,systemd-sysv,udev,dbus,\
apt-utils,locales,console-setup,keyboard-configuration,\
sudo,passwd,adduser,util-linux,procps,coreutils,\
findutils,grep,sed,gawk,curl,wget,ca-certificates,\
gnupg,lsb-release,squashfs-tools,rsync,pciutils,\
usbutils,dmidecode,hdparm,smartmontools,\
ntfs-3g,exfat-fuse,hfsplus,hfsutils,btrfs-progs,\
xfsprogs,dosfstools,parted,gdisk,fdisk,\
clamav,clamav-freshclam,yara,\
photorec,ddrescue,dc3dd,\
grub2-common,grub-pc-bin,grub-efi-amd64-bin,\
grub-efi-amd64-signed,shim-signed,\
nftables,iptables,\
bubblewrap,strace,\
openssh-client,\
python3-minimal,\
xz-utils,zstd,lz4,\
live-boot,live-config,\
casper,plymouth,plymouth-themes \
        noble \
        "${CHROOT_DIR}" \
        http://archive.ubuntu.com/ubuntu/

    ok "기본 시스템 설치 완료"

    # sources.list 설정
    cat > "${CHROOT_DIR}/etc/apt/sources.list" <<'EOF'
deb http://archive.ubuntu.com/ubuntu noble main restricted universe multiverse
deb http://archive.ubuntu.com/ubuntu noble-updates main restricted universe multiverse
deb http://security.ubuntu.com/ubuntu noble-security main restricted universe multiverse
EOF

    # 기본 설정
    chroot "${CHROOT_DIR}" locale-gen ko_KR.UTF-8 en_US.UTF-8 ja_JP.UTF-8
    chroot "${CHROOT_DIR}" update-locale LANG=en_US.UTF-8
    echo "AegisRescue" > "${CHROOT_DIR}/etc/hostname"

    # aegis 사용자 생성
    chroot "${CHROOT_DIR}" useradd -m -s /bin/bash -G sudo,cdrom,plugdev aegis 2>/dev/null || true
    echo "aegis:aegis" | chroot "${CHROOT_DIR}" chpasswd

    # 자동 로그인 (Live 모드)
    mkdir -p "${CHROOT_DIR}/etc/systemd/system/getty@tty1.service.d"
    cat > "${CHROOT_DIR}/etc/systemd/system/getty@tty1.service.d/autologin.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin aegis --noclear %I $TERM
EOF

    ok "기본 시스템 구성 완료"
}

# ─── STEP 2: 커널 설치 ────────────────────────────────────────
install_kernel() {
    step "STEP 2/6 — 커널 설치"

    if [[ $SKIP_KERNEL -eq 1 ]]; then
        ok "커널 빌드 건너뜀"
        return 0
    fi

    # 미리 빌드된 커널 사용 (Ubuntu HWE 커널)
    log "Ubuntu HWE 커널 설치 중..."
    mount_chroot
    chroot "${CHROOT_DIR}" apt-get install -y --no-install-recommends \
        linux-image-generic-hwe-24.04 \
        linux-headers-generic-hwe-24.04 \
        initramfs-tools \
        2>/dev/null || \
    chroot "${CHROOT_DIR}" apt-get install -y --no-install-recommends \
        linux-image-generic initramfs-tools
    umount_chroot

    # 커널 버전 감지
    KERNEL_VER=$(ls "${CHROOT_DIR}/boot/vmlinuz-"* 2>/dev/null | \
                 sort -V | tail -1 | xargs -I{} basename {} vmlinuz- 2>/dev/null || \
                 echo "generic")

    log "설치된 커널: ${KERNEL_VER}"
    ok "커널 설치 완료"
}

# ─── STEP 3: Rust 도구 빌드 ───────────────────────────────────
build_rust_tools() {
    step "STEP 3/6 — Rust 도구 빌드"

    if [[ $SKIP_RUST -eq 1 ]]; then
        ok "Rust 빌드 건너뜀"
        return 0
    fi

    command -v cargo &>/dev/null || {
        log "Rust 설치 중..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- -y --default-toolchain stable --no-modify-path
        source "${HOME}/.cargo/env"
    }

    # musl 타겟 추가 (정적 바이너리)
    if [[ "$ARCH" == "x86_64" ]]; then
        RUST_TARGET="x86_64-unknown-linux-musl"
        rustup target add "$RUST_TARGET" 2>/dev/null || true
        apt-get install -y musl-tools musl-dev 2>/dev/null || true
    elif [[ "$ARCH" == "aarch64" ]]; then
        RUST_TARGET="aarch64-unknown-linux-musl"
        rustup target add "$RUST_TARGET" 2>/dev/null || true
        apt-get install -y gcc-aarch64-linux-gnu musl-tools 2>/dev/null || true
    fi

    log "Rust 워크스페이스 빌드 중 (jobs=${JOBS})..."
    cd "${PROJECT_ROOT}"

    RUSTFLAGS="-C target-feature=+crt-static" \
    cargo build \
        --release \
        --jobs "${JOBS}" \
        --target "${RUST_TARGET}" \
        2>&1 | tee "${OUT_DIR}/rust-build.log" || {
        warn "musl 빌드 실패 — glibc 타겟으로 재시도"
        if [[ "$ARCH" == "x86_64" ]]; then
            RUST_TARGET="x86_64-unknown-linux-gnu"
        else
            RUST_TARGET="aarch64-unknown-linux-gnu"
        fi
        cargo build --release --jobs "${JOBS}" 2>&1 | tee "${OUT_DIR}/rust-build.log"
    }

    # 빌드된 바이너리 복사
    local release_dir="${PROJECT_ROOT}/target/${RUST_TARGET}/release"
    [[ -d "$release_dir" ]] || release_dir="${PROJECT_ROOT}/target/release"

    mkdir -p "${PROJECT_ROOT}/build/tools"
    local tools=(aegisd aegis aegis-scan aegis-mount aegis-recover
                 aegis-boot aegis-forensic aegis-image aegis-ai
                 aegis-sandbox aegis-preflight aegis-update)
    for tool in "${tools[@]}"; do
        if [[ -f "${release_dir}/${tool}" ]]; then
            cp "${release_dir}/${tool}" "${PROJECT_ROOT}/build/tools/"
            strip "${PROJECT_ROOT}/build/tools/${tool}" 2>/dev/null || true
            ok "  ${tool}"
        else
            warn "  ${tool} — 빌드 결과 없음"
        fi
    done

    ok "Rust 도구 빌드 완료"
}

# ─── STEP 4: GUI 빌드 ─────────────────────────────────────────
build_gui() {
    step "STEP 4/6 — GUI (AegisCenter + Installer) 빌드"

    if [[ $SKIP_GUI -eq 1 ]]; then
        ok "GUI 빌드 건너뜀"
        return 0
    fi

    command -v node &>/dev/null || {
        log "Node.js 설치 중..."
        curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -
        apt-get install -y nodejs
    }

    # AegisCenter GUI
    log "AegisCenter 빌드 중..."
    cd "${PROJECT_ROOT}/gui"
    npm install --prefer-offline 2>/dev/null || npm install
    npx tauri build 2>&1 | tee "${OUT_DIR}/gui-build.log" || warn "GUI 빌드 실패 (건너뜀)"
    if [[ -f "src-tauri/target/release/aegiscenter" ]]; then
        cp "src-tauri/target/release/aegiscenter" "${PROJECT_ROOT}/build/tools/"
        ok "AegisCenter 빌드 완료"
    fi

    # Installer GUI
    log "Installer 빌드 중..."
    cd "${PROJECT_ROOT}/installer"
    npm install --prefer-offline 2>/dev/null || npm install
    npx tauri build 2>&1 | tee -a "${OUT_DIR}/gui-build.log" || warn "Installer 빌드 실패 (건너뜀)"
    if [[ -f "src-tauri/target/release/aegis-installer" ]]; then
        cp "src-tauri/target/release/aegis-installer" "${PROJECT_ROOT}/build/tools/"
        ok "Installer 빌드 완료"
    fi
}

# ─── STEP 5: chroot에 파일 설치 ───────────────────────────────
populate_chroot() {
    step "STEP 5/6 — chroot 파일시스템 구성"

    mount_chroot

    # Aegis 도구 설치
    mkdir -p "${CHROOT_DIR}/opt/aegis/bin"
    if [[ -d "${PROJECT_ROOT}/build/tools" ]]; then
        cp -v "${PROJECT_ROOT}/build/tools/"* "${CHROOT_DIR}/opt/aegis/bin/" 2>/dev/null || true
    fi

    # PATH에 추가
    cat > "${CHROOT_DIR}/etc/profile.d/aegis.sh" <<'EOF'
export PATH="/opt/aegis/bin:$PATH"
EOF
    chmod 755 "${CHROOT_DIR}/etc/profile.d/aegis.sh"

    # 심볼릭 링크
    for bin in aegisd aegis aegiscenter aegis-installer aegis-preflight aegis-update; do
        [[ -f "${CHROOT_DIR}/opt/aegis/bin/${bin}" ]] && \
            ln -sf "/opt/aegis/bin/${bin}" "${CHROOT_DIR}/usr/bin/${bin}" 2>/dev/null || true
    done

    # 설정 파일
    mkdir -p "${CHROOT_DIR}/etc/aegis"
    cp "${PROJECT_ROOT}/packaging/policy/policy.toml" \
       "${CHROOT_DIR}/etc/aegis/policy.toml" 2>/dev/null || true

    # nftables (기본: Air-Gap)
    cp "${PROJECT_ROOT}/security/nftables/airgap.nft" \
       "${CHROOT_DIR}/etc/nftables.conf" 2>/dev/null || true

    # AppArmor 프로파일
    mkdir -p "${CHROOT_DIR}/etc/apparmor.d"
    cp "${PROJECT_ROOT}/security/apparmor/"*.apparmor \
       "${CHROOT_DIR}/etc/apparmor.d/" 2>/dev/null || true

    # 데이터 파일 (YARA, 시그니처)
    mkdir -p "${CHROOT_DIR}/var/lib/aegis/yara-rules"
    mkdir -p "${CHROOT_DIR}/var/lib/aegis/signatures"
    cp -r "${PROJECT_ROOT}/data/yara-rules/"* \
          "${CHROOT_DIR}/var/lib/aegis/yara-rules/" 2>/dev/null || true
    cp -r "${PROJECT_ROOT}/data/signatures/"* \
          "${CHROOT_DIR}/var/lib/aegis/signatures/" 2>/dev/null || true

    # 부팅 스플래시(Plymouth) 테마: 검은 배경 로고만 사용
    local logo_candidates=(
        "${AEGIS_BOOT_LOGO:-}"
        "${PROJECT_ROOT}/assets/aegisrescue-boot-logo-black.png"
        "/mnt/c/Users/fff20/.cursor/projects/f-ofntkd/assets/aegisrescue-boot-logo-black.png"
        "/c/Users/fff20/.cursor/projects/f-ofntkd/assets/aegisrescue-boot-logo-black.png"
    )
    local logo_src=""
    for candidate in "${logo_candidates[@]}"; do
        if [[ -n "$candidate" && -f "$candidate" ]]; then
            logo_src="$candidate"
            break
        fi
    done

    mkdir -p "${CHROOT_DIR}/usr/share/plymouth/themes/aegisrescue"
    cp "${PROJECT_ROOT}/build/plymouth/aegisrescue.plymouth" \
       "${CHROOT_DIR}/usr/share/plymouth/themes/aegisrescue/" 2>/dev/null || true
    cp "${PROJECT_ROOT}/build/plymouth/aegisrescue.script" \
       "${CHROOT_DIR}/usr/share/plymouth/themes/aegisrescue/" 2>/dev/null || true
    if [[ -n "$logo_src" ]]; then
        cp "$logo_src" "${CHROOT_DIR}/usr/share/plymouth/themes/aegisrescue/aegis-bg.png"
        mkdir -p "${CHROOT_DIR}/usr/share/aegis"
        cp "$logo_src" "${CHROOT_DIR}/usr/share/aegis/aegisrescue-boot-logo.png"
    fi
    chroot "${CHROOT_DIR}" update-alternatives --install \
        /usr/share/plymouth/themes/default.plymouth default.plymouth \
        /usr/share/plymouth/themes/aegisrescue/aegisrescue.plymouth 100 2>/dev/null || true
    chroot "${CHROOT_DIR}" update-alternatives --set default.plymouth \
        /usr/share/plymouth/themes/aegisrescue/aegisrescue.plymouth 2>/dev/null || true

    # Live 부팅 설정 (live-config)
    mkdir -p "${CHROOT_DIR}/etc/live"
    cat > "${CHROOT_DIR}/etc/live/config.conf" <<'EOF'
LIVE_HOSTNAME="aegisrescue"
LIVE_USERNAME="aegis"
LIVE_CONFIG_CMDLINE="false"
EOF

    # systemd 서비스
    mkdir -p "${CHROOT_DIR}/etc/systemd/system"
    for svc in aegisd aegis-preflight aegis-update aegis-installer \
               aegis-update.timer aegis-update.service; do
        [[ -f "${PROJECT_ROOT}/packaging/systemd/${svc}.service" ]] && \
            cp "${PROJECT_ROOT}/packaging/systemd/${svc}.service" \
               "${CHROOT_DIR}/etc/systemd/system/" 2>/dev/null || true
        [[ -f "${PROJECT_ROOT}/packaging/systemd/${svc}" ]] && \
            cp "${PROJECT_ROOT}/packaging/systemd/${svc}" \
               "${CHROOT_DIR}/etc/systemd/system/" 2>/dev/null || true
    done

    # 서비스 활성화
    # shellcheck source=configure-live-console.sh
    source "${SCRIPT_DIR}/configure-live-console.sh"
    configure_live_console "${CHROOT_DIR}"

    chroot "${CHROOT_DIR}" systemctl enable aegisd.service        2>/dev/null || true
    chroot "${CHROOT_DIR}" systemctl enable aegis-preflight.service 2>/dev/null || true
    chroot "${CHROOT_DIR}" systemctl enable aegis-update.timer    2>/dev/null || true

    # ClamAV 시그니처 DB 업데이트 (네트워크 있을 경우)
    if curl -s --max-time 3 https://database.clamav.net >/dev/null 2>&1; then
        log "ClamAV DB 업데이트 중..."
        chroot "${CHROOT_DIR}" freshclam 2>/dev/null || true
    fi

    # initramfs 재생성
    log "initramfs 재생성 중..."
    chroot "${CHROOT_DIR}" update-initramfs -u -k all 2>/dev/null || true

    umount_chroot
    ok "chroot 파일시스템 구성 완료"
}

# ─── STEP 6: squashfs + ISO 생성 ─────────────────────────────
build_iso() {
    step "STEP 6/6 — ISO 이미지 생성"

    mkdir -p "${OUT_DIR}"
    mkdir -p "${ISO_STAGE}"/{boot/grub,boot/grub/fonts,EFI/BOOT,live,isolinux}

    # squashfs 생성
    log "squashfs 압축 중 (xz, 시간이 걸립니다)..."
    mksquashfs "${CHROOT_DIR}" "${ISO_STAGE}/live/filesystem.squashfs" \
        -comp xz \
        -Xbcj x86 \
        -b 1M \
        -processors "${JOBS}" \
        -wildcards \
        -ef "${SCRIPT_DIR}/squashfs-excludes.txt" \
        -noappend \
        -progress \
        2>&1 | tail -5

    ok "squashfs 생성 완료: $(du -sh "${ISO_STAGE}/live/filesystem.squashfs" | cut -f1)"
    du -sx --block-size=1 "${CHROOT_DIR}" 2>/dev/null | cut -f1 \
        > "${ISO_STAGE}/live/filesystem.size" || true

    # 커널 / initramfs 복사
    local vmlinuz initrd
    vmlinuz=$(ls "${CHROOT_DIR}/boot/vmlinuz-"* 2>/dev/null | sort -V | tail -1)
    initrd=$(ls "${CHROOT_DIR}/boot/initrd.img-"* 2>/dev/null | sort -V | tail -1)

    if [[ -f "$vmlinuz" ]]; then
        cp "$vmlinuz" "${ISO_STAGE}/boot/vmlinuz"
        ok "커널: $(basename $vmlinuz)"
    else
        warn "커널 파일 없음 — 빌드를 먼저 실행하세요"
    fi
    if [[ -f "$initrd" ]]; then
        cp "$initrd" "${ISO_STAGE}/boot/initramfs.img"
        ok "initramfs: $(basename $initrd)"
    fi

    # filesystem.size 기록
    du -s --block-size=1 "${CHROOT_DIR}" | \
        cut -f1 > "${ISO_STAGE}/live/filesystem.size"

    # GRUB 설정
    cp "${SCRIPT_DIR}/grub/grub.cfg" "${ISO_STAGE}/boot/grub/grub.cfg"
    install_grub_fonts

    # ISOLINUX/SYSLINUX (BIOS 부팅)
    setup_syslinux

    # EFI 이미지 생성
    create_efi_image

    # ISO 생성
    log "xorriso로 하이브리드 ISO 생성 중..."
    xorriso -as mkisofs \
        -iso-level 3 \
        -full-iso9660-filenames \
        -volid "AEGISRESCUE_BETA_${VERSION//./_}" \
        -appid "${ISO_LABEL}" \
        -publisher "AegisRescue Project" \
        -preparer "AegisRescue Build System" \
        -isohybrid-mbr "${ISO_STAGE}/isolinux/isohdpfx.bin" \
        -eltorito-boot isolinux/isolinux.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
        -eltorito-catalog isolinux/boot.cat \
        -eltorito-alt-boot \
            -e EFI/BOOT/efiboot.img \
            -no-emul-boot \
        -isohybrid-gpt-basdat \
        -output "${ISO_NAME}" \
        "${ISO_STAGE}/" \
        2>&1 | grep -v "^$" | tail -20

    # SHA256 + MD5
    log "체크섬 생성 중..."
    cd "${OUT_DIR}"
    sha256sum "$(basename "${ISO_NAME}")" > "$(basename "${ISO_NAME}").sha256"
    md5sum    "$(basename "${ISO_NAME}")" > "$(basename "${ISO_NAME}").md5"

    local iso_size
    iso_size=$(du -sh "${ISO_NAME}" | cut -f1)

    echo ""
    echo -e "${GRN}${BLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RST}"
    echo -e "${GRN}${BLD}  ✓ AegisRescue ISO 빌드 완료!${RST}"
    echo -e "${GRN}${BLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RST}"
    echo -e "  ISO 파일: ${CYN}${ISO_NAME}${RST}"
    echo -e "  크기:     ${iso_size}"
    echo -e "  SHA256:   $(cat "${ISO_NAME}.sha256" | cut -d' ' -f1)"
    echo ""
    echo -e "  USB에 기록: ${YLW}sudo dd if=${ISO_NAME} of=/dev/sdX bs=4M status=progress${RST}"
    echo -e "  QEMU 테스트: ${YLW}make test-qemu${RST}"
    echo -e "${GRN}${BLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RST}"
}

# ─── 보조 함수 ────────────────────────────────────────────────

mount_chroot() {
    mount --bind /proc "${CHROOT_DIR}/proc"    2>/dev/null || true
    mount --bind /sys  "${CHROOT_DIR}/sys"     2>/dev/null || true
    mount --bind /dev  "${CHROOT_DIR}/dev"     2>/dev/null || true
    mount -t devpts devpts "${CHROOT_DIR}/dev/pts" 2>/dev/null || true
}

umount_chroot() {
    umount -l "${CHROOT_DIR}/dev/pts" 2>/dev/null || true
    umount -l "${CHROOT_DIR}/dev"     2>/dev/null || true
    umount -l "${CHROOT_DIR}/sys"     2>/dev/null || true
    umount -l "${CHROOT_DIR}/proc"    2>/dev/null || true
}

install_grub_fonts() {
    local grub_font_src
    grub_font_src=$(find /usr/share/grub -name "unicode.pf2" 2>/dev/null | head -1)
    if [[ -n "$grub_font_src" ]]; then
        cp "$grub_font_src" "${ISO_STAGE}/boot/grub/fonts/"
    fi
    # GRUB 테마
    mkdir -p "${ISO_STAGE}/boot/grub/themes/aegis"
    cp "${SCRIPT_DIR}/grub/theme.txt" \
       "${ISO_STAGE}/boot/grub/themes/aegis/" 2>/dev/null || true

    # 부팅 로고/배경: 검은 배경 단일 버전만 사용
    local logo_candidates=(
        "${AEGIS_BOOT_LOGO:-}"
        "${PROJECT_ROOT}/assets/aegisrescue-boot-logo-black.png"
        "/mnt/c/Users/fff20/.cursor/projects/f-ofntkd/assets/aegisrescue-boot-logo-black.png"
        "/c/Users/fff20/.cursor/projects/f-ofntkd/assets/aegisrescue-boot-logo-black.png"
    )
    local logo_src=""
    for candidate in "${logo_candidates[@]}"; do
        if [[ -n "$candidate" && -f "$candidate" ]]; then
            logo_src="$candidate"
            break
        fi
    done

    if [[ -n "$logo_src" ]]; then
        cp "$logo_src" "${ISO_STAGE}/boot/grub/aegis-bg.png"
        cp "$logo_src" "${ISO_STAGE}/boot/grub/themes/aegis/aegis-bg.png" 2>/dev/null || true
        ok "부팅 로고 적용: $(basename "$logo_src")"
    else
        warn "부팅 로고 파일을 찾지 못했습니다. AEGIS_BOOT_LOGO=/path/logo.png 지정 가능"
    fi
}

setup_syslinux() {
    # shellcheck source=stage-isolinux.sh
    source "${SCRIPT_DIR}/stage-isolinux.sh"
    # shellcheck source=configure-live-console.sh
    source "${SCRIPT_DIR}/configure-live-console.sh"

    local isolinux_rc=0
    stage_isolinux_files "${ISO_STAGE}" || isolinux_rc=$?
    if [[ ${isolinux_rc} -eq 0 ]]; then
        ok "SYSLINUX 파일 복사됨 (isolinux.bin + ldlinux.c32)"
    elif [[ ${isolinux_rc} -eq 2 ]]; then
        err "ldlinux.c32 없음 — apt install syslinux-common 후 ISO 재빌드"
    else
        warn "SYSLINUX 파일 없음 — BIOS 부팅 비활성화"
        dd if=/dev/zero bs=512 count=1 of="${ISO_STAGE}/isolinux/isohdpfx.bin" 2>/dev/null
        dd if=/dev/zero bs=512 count=4 of="${ISO_STAGE}/isolinux/isolinux.bin" 2>/dev/null
        touch "${ISO_STAGE}/isolinux/boot.cat"
    fi

    [[ -f "${ISO_STAGE}/isolinux/boot.cat" ]] || touch "${ISO_STAGE}/isolinux/boot.cat"

    # SYSLINUX 설정
    cat > "${ISO_STAGE}/isolinux/isolinux.cfg" <<EOF
DEFAULT vesamenu.c32
TIMEOUT 100
PROMPT 0
MENU TITLE AegisRescue Beta ${VERSION}

LABEL live
  MENU LABEL AegisRescue Live (Standard)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_BASE}

LABEL forensic
  MENU LABEL AegisRescue Forensic Mode
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_BASE} aegis.mode=forensic noswap

LABEL airgap
  MENU LABEL AegisRescue Air-Gap (Network Isolated)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_BASE} aegis.mode=airgap

LABEL toram
  MENU LABEL AegisRescue RAM Boot
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_BASE} toram

LABEL debug
  MENU LABEL Debug (verbose)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_BASE} debug verbose systemd.log_level=debug
EOF
}

create_efi_image() {
    log "EFI 부트 이미지 생성 중..."
    local efi_img="${ISO_STAGE}/EFI/BOOT/efiboot.img"
    local efi_size=10  # MiB

    dd if=/dev/zero bs=1M count=${efi_size} of="${efi_img}" 2>/dev/null
    mkfs.vfat "${efi_img}" 2>/dev/null || {
        warn "mkfs.vfat 실패 — EFI 이미지 생성 건너뜀"
        return 0
    }

    # GRUB EFI 파일 복사
    local grub_efi_dir
    grub_efi_dir=$(find /usr/lib/grub -name "grubx64.efi" 2>/dev/null | head -1 | xargs dirname)

    if [[ -z "$grub_efi_dir" ]]; then
        # grub-mkstandalone으로 생성
        if command -v grub-mkstandalone &>/dev/null; then
            grub-mkstandalone \
                --format=x86_64-efi \
                --output="${efi_img}.efi" \
                --locales="" \
                --themes="" \
                "boot/grub/grub.cfg=${SCRIPT_DIR}/grub/grub.cfg" \
                2>/dev/null || true
            if [[ -f "${efi_img}.efi" ]]; then
                mmd -i "${efi_img}" ::/EFI ::/EFI/BOOT 2>/dev/null || true
                mcopy -i "${efi_img}" "${efi_img}.efi" ::/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true
                rm -f "${efi_img}.efi"
            fi
        fi
    else
        mmd -i "${efi_img}" ::/EFI ::/EFI/BOOT 2>/dev/null || true
        mcopy -i "${efi_img}" "${grub_efi_dir}/grubx64.efi" ::/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true
    fi

    # Secure Boot shim
    local shim
    shim=$(find /usr/lib/shim -name "shimx64.efi.signed" 2>/dev/null | head -1)
    [[ -n "$shim" ]] && \
        mcopy -i "${efi_img}" "$shim" ::/EFI/BOOT/BOOTX64.EFI 2>/dev/null || true

    ok "EFI 이미지 생성 완료"
}

# ─── 메인 진입점 ──────────────────────────────────────────────
main() {
    echo -e "${BLD}"
    echo "  ╔═══════════════════════════════════════════════════════╗"
    echo "  ║          AegisRescue ISO 빌드 시스템                  ║"
    echo "  ║  버전: ${VERSION}  아키텍처: ${ARCH}  빌드 코어: ${JOBS}              ║"
    echo "  ╚═══════════════════════════════════════════════════════╝"
    echo -e "${RST}"

    check_root
    check_deps

    mkdir -p "${OUT_DIR}"
    trap 'umount_chroot 2>/dev/null || true' EXIT

    local start_time=$SECONDS

    [[ $ONLY_ISO -eq 0 ]] && build_base_system
    [[ $ONLY_ISO -eq 0 ]] && install_kernel
    [[ $ONLY_ISO -eq 0 ]] && build_rust_tools
    [[ $ONLY_ISO -eq 0 ]] && build_gui
    [[ $ONLY_ISO -eq 0 ]] && populate_chroot
    build_iso

    local elapsed=$(( SECONDS - start_time ))
    echo -e "\n⏱ 총 빌드 시간: $((elapsed/60))분 $((elapsed%60))초"
}

main "$@"
