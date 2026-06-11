#!/usr/bin/env bash
# =============================================================================
# AegisRescue — WSL2 전용 ISO 빌드 스크립트
# =============================================================================
# 사전 조건:
#   1. Windows 10 21H2+ / Windows 11
#   2. WSL2 활성화 (Ubuntu 22.04 또는 24.04 권장)
#   3. 이 스크립트를 WSL 내부 파일시스템에서 실행 (중요!)
#
# 사용법:
#   # Windows PowerShell에서:
#   wsl bash /mnt/f/ofntkd/aegisrescue/build/wsl-build.sh --setup
#
#   # WSL 터미널에서:
#   cd ~/aegisrescue
#   sudo bash build/wsl-build.sh [옵션]
#
# 옵션:
#   --setup          WSL 환경 초기 설정 (한 번만 실행)
#   --copy           Windows 파일을 WSL 홈으로 복사
#   --arch ARCH      x86_64 | aarch64 (기본: x86_64)
#   --ver  VERSION   버전 (기본: 0.1.0)
#   --jobs N         병렬 빌드 수 (기본: nproc)
#   --skip-rust      Rust 빌드 건너뜀
#   --skip-gui       GUI 빌드 건너뜀
#   --skip-base      debootstrap 건너뜀 (기존 chroot 재사용)
#   --iso-only       squashfs + ISO만 재생성
#   --qemu-test      빌드 후 QEMU로 자동 테스트
# =============================================================================
set -euo pipefail

# ─── 기본값 ────────────────────────────────────────────────────
ARCH="${ARCH:-x86_64}"
VERSION="${VERSION:-0.1.0}"
RELEASE="${RELEASE:-beta}"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
WSL_HOME="${HOME}/aegisrescue"
WIN_SRC="/mnt/f/ofntkd/aegisrescue"   # Windows 소스 경로 (마운트 경로)
OUT_DIR="${WSL_HOME}/build/out"
CHROOT_DIR="${WSL_HOME}/build/chroot"
ISO_STAGE="${WSL_HOME}/build/iso-stage"
TOOLS_DIR="${WSL_HOME}/build/tools"
ISO_BASENAME="aegisrescue-${RELEASE}-${VERSION}-${ARCH}"
ISO_NAME="${OUT_DIR}/${ISO_BASENAME}.iso"
ISO_LABEL="AegisRescue Beta ${VERSION}"

DO_SETUP=0; DO_COPY=0; SKIP_RUST=0; SKIP_GUI=0
SKIP_BASE=0; ISO_ONLY=0; QEMU_TEST=0

# Never allow apt/debconf packages such as tzdata to stop the build.
export DEBIAN_FRONTEND="${DEBIAN_FRONTEND:-noninteractive}"
export TZ="${TZ:-Etc/UTC}"

# ─── 색상 ──────────────────────────────────────────────────────
R='\033[0;31m'; G='\033[0;32m'; Y='\033[1;33m'
C='\033[0;36m'; B='\033[1m'; N='\033[0m'

# echo -e treats \a \b etc. as escapes — use forward slashes for Windows paths.
win_display_path() {
    local p="${1:-${WIN_SRC}}"
    if [[ "${p}" =~ ^/mnt/([a-z])/(.*)$ ]]; then
        printf '%s:/%s' "${BASH_REMATCH[1]^^}" "${BASH_REMATCH[2]}"
    else
        printf '%s' "${p}"
    fi
}

log()  { echo -e "${C}[$(date +%H:%M:%S)]${N} $*"; }
ok()   { echo -e "${G}✓${N} $*"; }
warn() { echo -e "${Y}⚠${N} $*"; }
err()  { echo -e "${R}✗ FATAL:${N} $*" >&2; exit 1; }
step() { echo -e "\n${B}━━━ $* ━━━${N}"; }

# ─── 인수 파싱 ──────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --setup)      DO_SETUP=1;   shift ;;
        --copy)       DO_COPY=1;    shift ;;
        --arch)       ARCH="$2";    shift 2 ;;
        --ver)        VERSION="$2"; shift 2 ;;
        --jobs)       JOBS="$2";    shift 2 ;;
        --skip-rust)  SKIP_RUST=1;  shift ;;
        --skip-gui)   SKIP_GUI=1;   shift ;;
        --skip-base)  SKIP_BASE=1;  shift ;;
        --iso-only)   ISO_ONLY=1; SKIP_RUST=1; SKIP_GUI=1; SKIP_BASE=1; shift ;;
        --qemu-test)  QEMU_TEST=1;  shift ;;
        -h|--help)    grep '^#' "$0" | head -25 | sed 's/^# \?//'; exit 0 ;;
        *) err "알 수 없는 옵션: $1" ;;
    esac
done

# ─── WSL 환경 감지 ──────────────────────────────────────────────
detect_wsl() {
    if ! grep -qi "microsoft\|wsl" /proc/version 2>/dev/null; then
        err "WSL 환경이 아닙니다. 이 스크립트는 WSL2에서 실행해야 합니다."
    fi

    # WSL2 usually exposes WSL_INTEROP. Older detection based on "Microsoft"
    # in osrelease incorrectly warns on normal WSL2 kernels.
    local has_wsl2_interop=0
    if [[ -n "${WSL_INTEROP:-}" ]]; then
        has_wsl2_interop=1
    elif compgen -G "/run/WSL/*_interop" >/dev/null 2>&1; then
        has_wsl2_interop=1
    fi

    if [[ "${has_wsl2_interop}" -eq 0 ]]; then
        warn "WSL1 감지 — WSL2 업그레이드를 권장합니다"
        warn "PowerShell: wsl --set-version Ubuntu 2"
    fi
    log "WSL 환경 확인됨"

    # Windows 드라이브 마운트 경로 자동 탐지
    # F:\ofntkd\aegisrescue → /mnt/f/ofntkd/aegisrescue
    local win_drive
    win_drive=$(echo "${WIN_SRC}" | cut -c6)  # /mnt/f → f
    if [[ ! -d "/mnt/${win_drive}" ]]; then
        warn "Windows 드라이브 /mnt/${win_drive} 없음 — 직접 경로 확인 필요"
        # 자동 감지 시도
        for mnt in /mnt/[c-z]; do
            if [[ -d "${mnt}/ofntkd/aegisrescue" ]]; then
                WIN_SRC="${mnt}/ofntkd/aegisrescue"
                ok "소스 경로 자동 감지: ${WIN_SRC}"
                break
            fi
        done
    fi
}

# ─── STEP 0: WSL 환경 초기 설정 ─────────────────────────────────
setup_wsl() {
    step "WSL 환경 초기 설정"

    # WSL 설정 파일 최적화
    cat > /etc/wsl.conf.tmp <<'EOF'
[boot]
systemd=true

[wsl2]
memory=6GB
processors=4
swap=2GB
localhostForwarding=true

[automount]
enabled=true
options=metadata,uid=1000,gid=1000,umask=022

[interop]
enabled=true
appendWindowsPath=false
EOF

    if [[ -f /etc/wsl.conf ]]; then
        warn "/etc/wsl.conf 이미 존재 — 수동으로 병합하세요"
        echo "--- 권장 설정 ---"
        cat /etc/wsl.conf.tmp
        rm /etc/wsl.conf.tmp
    else
        mv /etc/wsl.conf.tmp /etc/wsl.conf
        ok "/etc/wsl.conf 생성됨"
    fi

    # 빌드 의존성 설치
    log "시스템 패키지 업데이트 및 설치 중..."
    apt-get update -qq
    apt-get install -y --no-install-recommends \
        # ISO / 파일시스템 도구
        debootstrap xorriso squashfs-tools \
        grub-pc-bin grub-efi-amd64-bin grub-efi-amd64-signed \
        grub2-common grub-common \
        isolinux syslinux syslinux-common \
        dosfstools mtools \
        # 압축
        xz-utils zstd lz4 gzip \
        # 유틸
        rsync pv curl wget ca-certificates \
        imagemagick librsvg2-bin \
        git build-essential pkg-config libssl-dev \
        # musl 정적 바이너리
        musl-tools musl-dev \
        # Tauri 공통 (WebKitGTK는 ensure_tauri_deps에서 Ubuntu 버전별 처리)
        libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
        # QEMU 테스트
        qemu-system-x86 ovmf \
        # 기타
        locales sudo gpg apt-transport-https \
        2>/dev/null | grep -E "^(Setting up|Get:|Unpacking)" | head -30 || true

    ok "패키지 설치 완료"
    ensure_tauri_deps || warn "WebKitGTK 설치 실패 — GUI 빌드는 나중에 다시 시도하세요"

    # Rust 설치
    if ! command -v cargo &>/dev/null; then
        log "Rust 설치 중..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- -y --default-toolchain stable --no-modify-path
        source "${HOME}/.cargo/env"
        ok "Rust $(rustc --version) 설치됨"
    else
        ok "Rust $(rustc --version) 이미 설치됨"
        rustup update stable 2>/dev/null | tail -1
    fi

    # musl 타겟 추가
    "${HOME}/.cargo/bin/rustup" target add x86_64-unknown-linux-musl 2>/dev/null || true
    "${HOME}/.cargo/bin/rustup" target add aarch64-unknown-linux-musl 2>/dev/null || true

    # Node.js 설치
    if ! command -v node &>/dev/null; then
        log "Node.js 설치 중..."
        curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -
        apt-get install -y nodejs
        ok "Node.js $(node --version) 설치됨"
    else
        ok "Node.js $(node --version) 이미 설치됨"
    fi

    # WSL 빌드 전용 설정
    # WSL에서 loop 디바이스 활성화
    modprobe loop 2>/dev/null || warn "loop 모듈 로드 실패 (WSL 제한)"
    modprobe squashfs 2>/dev/null || true
    modprobe overlay 2>/dev/null || true

    echo ""
    ok "WSL 환경 설정 완료!"
    echo ""
    echo -e "${Y}다음 단계:${N}"
    echo "  1. WSL 재시작: PowerShell에서 → wsl --shutdown && wsl"
    echo "  2. 소스 복사:  sudo bash build/wsl-build.sh --copy"
    echo "  3. 전체 빌드:  sudo bash build/wsl-build.sh"
}

# ─── STEP 1: Windows → WSL 파일시스템 복사 ──────────────────────
copy_source() {
    step "Windows 소스 → WSL 파일시스템 복사"

    if [[ ! -d "${WIN_SRC}" ]]; then
        err "Windows 소스 경로 없음: ${WIN_SRC}\n경로를 확인하세요: ls /mnt/f/"
    fi

    log "복사 중: ${WIN_SRC} → ${WSL_HOME}"
    log "참고: Windows 파일시스템(/mnt/f/)은 빌드 속도가 느려 WSL 홈으로 복사합니다"

    mkdir -p "${WSL_HOME}"
    rsync -a --info=progress2 \
        --exclude='.git/' \
        --exclude='target/' \
        --exclude='node_modules/' \
        --exclude='build/chroot/' \
        --exclude='build/iso-stage/' \
        --exclude='build/out/' \
        --exclude='*.o' \
        "${WIN_SRC}/" "${WSL_HOME}/"

    ok "복사 완료: ${WSL_HOME}"
    ok "총 크기: $(du -sh "${WSL_HOME}" | cut -f1)"

    # 스크립트 실행 권한 설정
    chmod +x "${WSL_HOME}/build/build.sh" \
             "${WSL_HOME}/build/wsl-build.sh" \
             "${WSL_HOME}/build/render-boot-logo.sh" \
             "${WSL_HOME}/build/stage-boot-logos.sh" \
             "${WSL_HOME}/build/grub-efi-standalone.sh" \
             "${WSL_HOME}/build/initramfs/hooks/aegis" \
             "${WSL_HOME}/build/initramfs/scripts/live-bottom/aegis-mode" \
             2>/dev/null || true
}

# ─── STEP 2: WSL 마운트 헬퍼 (--bind 대체) ──────────────────────
wsl_mount_chroot() {
    local chroot="$1"
    # WSL2에서 mount --bind는 제한적으로 동작
    # systemd-nspawn 또는 직접 마운트 시도
    mount --bind /proc "${chroot}/proc" 2>/dev/null || \
        mount -t proc proc "${chroot}/proc" 2>/dev/null || true
    mount --bind /sys  "${chroot}/sys"  2>/dev/null || \
        mount -t sysfs sysfs "${chroot}/sys" 2>/dev/null || true
    mount --bind /dev  "${chroot}/dev"  2>/dev/null || \
        mount -t devtmpfs devtmpfs "${chroot}/dev" 2>/dev/null || true
    mount -t devpts devpts "${chroot}/dev/pts" \
        -o gid=5,mode=620 2>/dev/null || true
    mount -t tmpfs tmpfs "${chroot}/run" 2>/dev/null || true
}

wsl_umount_chroot() {
    local chroot="$1"
    for mp in dev/pts dev sys proc run; do
        umount -l "${chroot}/${mp}" 2>/dev/null || true
    done
}

# ─── WSL 전용 debootstrap ────────────────────────────────────────
build_base_wsl() {
    step "기본 시스템 구성 (debootstrap + WSL 최적화)"

    if [[ $SKIP_BASE -eq 1 && -d "${CHROOT_DIR}" ]]; then
        ok "기존 chroot 재사용"
        return 0
    fi

    [[ -d "${CHROOT_DIR}" ]] && rm -rf "${CHROOT_DIR}"
    mkdir -p "${CHROOT_DIR}"

    log "Ubuntu 24.04 minimal 설치 중..."
    log "(약 5-10분 소요, 인터넷 연결 필요)"

    local debootstrap_log="${OUT_DIR}/debootstrap.log"
    mkdir -p "${OUT_DIR}"

    if ! debootstrap \
        --arch=amd64 \
        --variant=minbase \
        --components=main,restricted,universe,multiverse \
        --include=systemd,systemd-sysv,udev,dbus,\
apt-utils,locales,console-setup,\
sudo,passwd,adduser,util-linux,procps,coreutils,\
findutils,grep,sed,gawk,curl,wget,ca-certificates,\
gnupg,lsb-release,python3-minimal,xz-utils,zstd,lz4 \
        noble \
        "${CHROOT_DIR}" \
        http://archive.ubuntu.com/ubuntu/ \
        >"${debootstrap_log}" 2>&1; then
        warn "debootstrap 실패. 마지막 로그:"
        tail -40 "${debootstrap_log}" || true
        err "베이스 시스템 생성 실패: ${debootstrap_log}"
    fi

    ok "debootstrap 완료"

    # chroot 내 설정
    wsl_mount_chroot "${CHROOT_DIR}"

    # sources.list
    mkdir -p "${CHROOT_DIR}/etc/apt"
    cat > "${CHROOT_DIR}/etc/apt/sources.list" <<'EOF'
deb http://archive.ubuntu.com/ubuntu noble main restricted universe multiverse
deb http://archive.ubuntu.com/ubuntu noble-updates main restricted universe multiverse
deb http://security.ubuntu.com/ubuntu noble-security main restricted universe multiverse
EOF

    # 호스트명 / 로케일
    echo "aegisrescue" > "${CHROOT_DIR}/etc/hostname"
    echo "Etc/UTC" > "${CHROOT_DIR}/etc/timezone"
    mkdir -p "${CHROOT_DIR}/etc"
    ln -sf /usr/share/zoneinfo/Etc/UTC "${CHROOT_DIR}/etc/localtime" 2>/dev/null || true
    chroot "${CHROOT_DIR}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get update -qq 2>/dev/null || true
    chroot "${CHROOT_DIR}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get install -y --no-install-recommends \
        pciutils usbutils ntfs-3g exfat-fuse hfsplus hfsutils btrfs-progs \
        xfsprogs dosfstools parted gdisk fdisk \
        clamav yara testdisk gddrescue dc3dd \
        grub2-common grub-pc-bin grub-efi-amd64-bin grub-efi-amd64-signed shim-signed \
        nftables iptables bubblewrap strace \
        live-boot live-config live-config-systemd casper \
        plymouth plymouth-themes \
        2>&1 | tee "${OUT_DIR}/chroot-packages.log" | tail -20 || \
        warn "일부 선택 패키지 설치 실패. 계속 진행합니다: ${OUT_DIR}/chroot-packages.log"

    chroot "${CHROOT_DIR}" locale-gen ko_KR.UTF-8 en_US.UTF-8 ja_JP.UTF-8 2>/dev/null || true
    chroot "${CHROOT_DIR}" update-locale LANG=en_US.UTF-8 2>/dev/null || true

    # Live 부팅 설정 (live-config)
    mkdir -p "${CHROOT_DIR}/etc/live"
    cat > "${CHROOT_DIR}/etc/live/config.conf" <<'EOF'
LIVE_HOSTNAME="aegisrescue"
LIVE_USERNAME="aegis"
LIVE_CONFIG_CMDLINE="false"
EOF

    # aegis 사용자
    chroot "${CHROOT_DIR}" useradd -m -s /bin/bash -G sudo aegis 2>/dev/null || true
    echo "aegis:aegis" | chroot "${CHROOT_DIR}" chpasswd 2>/dev/null || true

    # 자동 로그인 설정
    mkdir -p "${CHROOT_DIR}/etc/systemd/system/getty@tty1.service.d"
    cat > "${CHROOT_DIR}/etc/systemd/system/getty@tty1.service.d/autologin.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin aegis --noclear %I $TERM
EOF

    # HWE 커널 설치
    log "커널 설치 중..."
    chroot "${CHROOT_DIR}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get update -qq 2>/dev/null || true
    chroot "${CHROOT_DIR}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get install -y --no-install-recommends \
        linux-image-generic initramfs-tools \
        2>/dev/null | grep -E "^(Setting up|Unpacking)" | head -10 || true

    wsl_umount_chroot "${CHROOT_DIR}"
    ok "기본 시스템 구성 완료"
}

# ─── Rust 빌드 (WSL 최적화) ──────────────────────────────────────
build_rust_wsl() {
    step "Rust 도구 빌드"
    [[ $SKIP_RUST -eq 1 ]] && { ok "건너뜀"; return 0; }

    # PATH에 cargo 추가
    source "${HOME}/.cargo/env" 2>/dev/null || \
    export PATH="${HOME}/.cargo/bin:${PATH}"

    if ! command -v cargo >/dev/null 2>&1; then
        log "Rust/Cargo가 없어 root 환경에 설치 중..."
        apt-get update -qq 2>/dev/null || true
        apt-get install -y --no-install-recommends \
            curl ca-certificates build-essential pkg-config libssl-dev \
            musl-tools musl-dev 2>/dev/null || true
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- -y --default-toolchain stable --no-modify-path
        source "${HOME}/.cargo/env"
    fi

    local rust_target="x86_64-unknown-linux-musl"
    [[ "$ARCH" == "aarch64" ]] && rust_target="aarch64-unknown-linux-musl"
    rustup target add "${rust_target}" 2>/dev/null || true

    log "빌드 타겟: ${rust_target} (jobs=${JOBS})"
    log "※ WSL 홈 파일시스템에서 빌드 중 (빠른 I/O)"

    cd "${WSL_HOME}"
    mkdir -p "${TOOLS_DIR}"

    # musl 빌드 시도, 실패 시 glibc 폴백
    # NOTE: grep/tail in a pipeline must not decide success — pipefail + set -e
    # would exit the whole build when grep finds no matches.
    local release_dir cargo_status=1
    : > "${OUT_DIR}/rust-build.log"

    RUSTFLAGS="-C target-feature=+crt-static" \
        cargo build --release \
            --jobs "${JOBS}" \
            --target "${rust_target}" \
            2>&1 | tee "${OUT_DIR}/rust-build.log" | tail -20
    cargo_status=${PIPESTATUS[0]}

    if [[ "${cargo_status}" -eq 0 ]]; then
        release_dir="${WSL_HOME}/target/${rust_target}/release"
    else
        warn "musl 빌드 실패 — glibc 타겟으로 재시도"
        rust_target="${ARCH}-unknown-linux-gnu"
        cargo build --release --jobs "${JOBS}" \
            2>&1 | tee -a "${OUT_DIR}/rust-build.log" | tail -20
        cargo_status=${PIPESTATUS[0]}
        release_dir="${WSL_HOME}/target/release"
    fi

    if [[ "${cargo_status}" -ne 0 ]]; then
        err "Rust 빌드 실패. 로그 확인: ${OUT_DIR}/rust-build.log"
    fi

    # 바이너리 복사
    local tools=(aegisd aegis aegis-scan aegis-mount aegis-recover
                 aegis-boot aegis-forensic aegis-image aegis-ai
                 aegis-sandbox aegis-preflight aegis-update)
    for tool in "${tools[@]}"; do
        local src="${release_dir}/${tool}"
        if [[ -f "$src" ]]; then
            cp "$src" "${TOOLS_DIR}/"
            strip "${TOOLS_DIR}/${tool}" 2>/dev/null || true
            printf "  ✓ %-25s %s\n" "$tool" "$(du -sh "${TOOLS_DIR}/${tool}" | cut -f1)"
        else
            warn "  ${tool} — 빌드 결과 없음"
        fi
    done

    ok "Rust 빌드 완료"
}

ensure_cargo() {
    source "${HOME}/.cargo/env" 2>/dev/null || \
    export PATH="${HOME}/.cargo/bin:${PATH}"

    if command -v cargo >/dev/null 2>&1; then
        return 0
    fi

    log "Rust/Cargo 없음 — GUI 빌드용 설치 중..."
    apt-get update -qq 2>/dev/null || true
    apt-get install -y --no-install-recommends \
        curl ca-certificates build-essential pkg-config libssl-dev \
        2>/dev/null || true
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
        sh -s -- -y --default-toolchain stable --no-modify-path
    source "${HOME}/.cargo/env"
    command -v cargo >/dev/null 2>&1
}

tauri_pkg_config_path() {
    local paths=()
    local dir
    for dir in \
        /usr/lib/x86_64-linux-gnu/pkgconfig \
        /usr/lib/pkgconfig \
        /usr/share/pkgconfig \
        /usr/local/lib/pkgconfig; do
        [[ -d "${dir}" ]] && paths+=("${dir}")
    done
    if ((${#paths[@]} == 0)); then
        echo ""
        return 0
    fi
    local IFS=:
    echo "${paths[*]}"
}

tauri_export_pkg_config() {
    local pc_path
    pc_path="$(tauri_pkg_config_path)"
    [[ -n "${pc_path}" ]] && export PKG_CONFIG_PATH="${pc_path}${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"
    export PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1
    export PKG_CONFIG_ALLOW_SYSTEM_LIBS=1
}

tauri_webkit4_pkg_ready() {
    tauri_export_pkg_config
    pkg-config --exists javascriptcoregtk-4.0 2>/dev/null && \
        pkg-config --exists webkit2gtk-4.0 2>/dev/null
}

tauri_webkit4_link_ready() {
    local lib_dir="/usr/lib/x86_64-linux-gnu"
    [[ -e "${lib_dir}/libwebkit2gtk-4.0.so" || -e "${lib_dir}/libwebkit2gtk-4.0.so.0" ]] && \
        [[ -e "${lib_dir}/libjavascriptcoregtk-4.0.so" || -e "${lib_dir}/libjavascriptcoregtk-4.0.so.0" ]]
}

tauri_webkit4_ready() {
    tauri_webkit4_pkg_ready && tauri_webkit4_link_ready
}

tauri_link_webkit41_libs_as_40() {
    local lib_dir="/usr/lib/x86_64-linux-gnu"
    [[ -d "${lib_dir}" ]] || return 1

    local pair src dst
    for pair in \
        "libwebkit2gtk-4.1.so:libwebkit2gtk-4.0.so" \
        "libwebkit2gtk-4.1.so.0:libwebkit2gtk-4.0.so.0" \
        "libjavascriptcoregtk-4.1.so:libjavascriptcoregtk-4.0.so" \
        "libjavascriptcoregtk-4.1.so.0:libjavascriptcoregtk-4.0.so.0"; do
        src="${lib_dir}/${pair%%:*}"
        dst="${lib_dir}/${pair##*:}"
        [[ -e "${src}" ]] || continue
        ln -sf "$(basename "${src}")" "${dst}"
        log "  lib: ${dst} -> $(basename "${src}")"
    done
}

tauri_link_webkit41_as_40() {
    local pc_dir src dst
    for pc_dir in /usr/lib/x86_64-linux-gnu/pkgconfig /usr/lib/pkgconfig; do
        [[ -d "${pc_dir}" ]] || continue
        for pair in \
            "javascriptcoregtk-4.1.pc:javascriptcoregtk-4.0.pc" \
            "webkit2gtk-4.1.pc:webkit2gtk-4.0.pc" \
            "webkit2gtk-web-extension-4.1.pc:webkit2gtk-web-extension-4.0.pc"; do
            src="${pc_dir}/${pair%%:*}"
            dst="${pc_dir}/${pair##*:}"
            [[ -f "${src}" ]] || continue
            ln -sf "$(basename "${src}")" "${dst}"
            log "  pkg-config: ${dst} -> $(basename "${src}")"
        done
    done
}

ensure_tauri_icons() {
    local gen_script="${WSL_HOME}/build/gen-tauri-icons.py"
    if command -v python3 &>/dev/null && [[ -f "${gen_script}" ]]; then
        log "Tauri RGBA 아이콘 생성 중..."
        python3 "${gen_script}" --root "${WSL_HOME}" || return 1
        return 0
    fi

    local icon="${WSL_HOME}/gui/src-tauri/icons/32x32.png"
    if [[ ! -f "${icon}" ]]; then
        warn "Tauri 아이콘 없음 — python3 또는 --copy 필요"
        return 1
    fi

    # PNG IHDR color type byte must be 6 (RGBA)
    local ctype
    ctype=$(python3 -c "print(open('${icon}','rb').read()[25])" 2>/dev/null || echo 0)
    if [[ "${ctype}" != "6" ]]; then
        warn "Tauri 아이콘이 RGBA가 아님 — python3로 gen-tauri-icons.py 실행 또는 --copy"
        return 1
    fi
    return 0
}

tauri_log_webkit_status() {
    local log_file="${1:-}"
    {
        echo "PKG_CONFIG_PATH=${PKG_CONFIG_PATH:-<unset>}"
        pkg-config --modversion javascriptcoregtk-4.0 2>&1 || true
        pkg-config --modversion webkit2gtk-4.0 2>&1 || true
        ls -la /usr/lib/x86_64-linux-gnu/pkgconfig/javascriptcoregtk*.pc 2>&1 || true
        ls -la /usr/lib/x86_64-linux-gnu/pkgconfig/webkit2gtk*.pc 2>&1 || true
        ls -la /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.* 2>&1 || true
        ls -la /usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-4.* 2>&1 || true
    } >> "${log_file}" 2>&1
}

ensure_tauri_deps() {
    apt-get update -qq 2>/dev/null || true
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
        libssl-dev patchelf pkg-config libsoup2.4-dev \
        >/dev/null 2>&1 || true

    if tauri_webkit4_pkg_ready && tauri_webkit4_link_ready; then
        ok "WebKitGTK 4.0 (Tauri v1) 준비됨"
        return 0
    fi

    log "WebKitGTK 4.0 설치 중 (Tauri v1 필수)..."

    # Ubuntu 22.04 / Debian 12: 네이티브 4.0 패키지
    if apt-cache show libwebkit2gtk-4.0-dev &>/dev/null; then
        DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
            libwebkit2gtk-4.0-dev >/dev/null 2>&1 || true
        if tauri_webkit4_pkg_ready && tauri_webkit4_link_ready; then
            ok "libwebkit2gtk-4.0-dev 설치됨"
            return 0
        fi
    fi

    # Ubuntu 24.04+: 4.1만 제공 → pkg-config + shared lib 4.0 별칭 (Tauri v1 우회)
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev \
        >/dev/null 2>&1 || true
    tauri_link_webkit41_as_40
    tauri_link_webkit41_libs_as_40
    if tauri_webkit4_pkg_ready && tauri_webkit4_link_ready; then
        ok "WebKitGTK 4.1 + 4.0 별칭 (Ubuntu 24+)"
        return 0
    fi

    # 최후 수단: Jammy 저장소에서 4.0 설치 (핀 고정으로 시스템 패키지 보호)
    log "Jammy 저장소에서 libwebkit2gtk-4.0-dev 설치 시도..."
    local jammy_list="/etc/apt/sources.list.d/aegis-webkit-jammy.list"
    local jammy_pin="/etc/apt/preferences.d/aegis-webkit-jammy"
    echo "deb [arch=amd64] http://archive.ubuntu.com/ubuntu jammy main universe" > "${jammy_list}"
    cat > "${jammy_pin}" <<'EOF'
Package: libwebkit2gtk-4.0-dev libjavascriptcoregtk-4.0-dev
Pin: release n=jammy
Pin-Priority: 700

Package: *
Pin: release n=jammy
Pin-Priority: 100
EOF
    apt-get update -qq 2>/dev/null || true
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libwebkit2gtk-4.0-dev >/dev/null 2>&1 || true
    rm -f "${jammy_list}" "${jammy_pin}"
    apt-get update -qq 2>/dev/null || true
    if tauri_webkit4_pkg_ready && tauri_webkit4_link_ready; then
        ok "Jammy WebKitGTK 4.0 설치됨"
        return 0
    fi

    warn "javascriptcoregtk-4.0 / webkit2gtk-4.0 (pkg-config 또는 shared lib) 를 찾을 수 없습니다"
    warn "수동 확인: pkg-config --exists javascriptcoregtk-4.0"
    warn "수동 설치: sudo bash build/wsl-build.sh --setup"
    return 1
}

ensure_nodejs() {
    if command -v npm &>/dev/null && command -v node &>/dev/null; then
        ok "Node.js $(node --version) / npm $(npm --version) 사용"
        return 0
    fi

    log "Node.js/npm 없음 — 설치 중..."
    apt-get update -qq 2>/dev/null || true

    if apt-get install -y --no-install-recommends nodejs npm \
        2>/dev/null | grep -E "^(Setting up|Unpacking)" | tail -5; then
        :
    else
        curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - 2>/dev/null || true
        apt-get install -y nodejs 2>/dev/null || true
    fi

    if command -v npm &>/dev/null && command -v node &>/dev/null; then
        ok "Node.js $(node --version) / npm $(npm --version) 설치됨"
        return 0
    fi

    warn "Node.js/npm 설치 실패"
    return 1
}

find_tauri_binary() {
    local project_dir="$1"
    local bin="$2"
    local candidate found

    for candidate in \
        "${project_dir}/src-tauri/target/release/${bin}" \
        "${project_dir}/target/release/${bin}"; do
        [[ -f "${candidate}" ]] && { echo "${candidate}"; return 0; }
    done

    found=$(find "${project_dir}/src-tauri/target/release" -type f -name "${bin}" 2>/dev/null | head -1)
    [[ -n "${found}" ]] && { echo "${found}"; return 0; }

    found=$(find "${project_dir}/src-tauri/target/release/bundle" -type f \( \
        -name "${bin}" -o -path "*/usr/bin/${bin}" -o -path "*/data/usr/bin/${bin}" \
        \) 2>/dev/null | head -1)
    [[ -n "${found}" ]] && { echo "${found}"; return 0; }

    found=$(find "${project_dir}/src-tauri/target/release/bundle/deb" -type f -path "*/usr/bin/*" ! -name "*.png" 2>/dev/null | head -1)
    [[ -n "${found}" ]] && { echo "${found}"; return 0; }
    return 1
}

tauri_pick_usr_bin() {
    local root="$1"
    local preferred="$2"
    local found

    for candidate in \
        "${root}/usr/bin/${preferred}" \
        "${root}/data/usr/bin/${preferred}"; do
        [[ -f "${candidate}" ]] && { echo "${candidate}"; return 0; }
    done

    found=$(find "${root}" -type f \( -path "*/usr/bin/${preferred}" -o -path "*/data/usr/bin/${preferred}" \) 2>/dev/null | head -1)
    [[ -n "${found}" ]] && { echo "${found}"; return 0; }

    found=$(find "${root}" -type f \( -path "*/usr/bin/*" -o -path "*/data/usr/bin/*" \) ! -name "*.png" 2>/dev/null | head -1)
    [[ -n "${found}" ]] && { echo "${found}"; return 0; }
    return 1
}

extract_tauri_binary_from_deb() {
    local project_dir="$1"
    local bin="$2"
    local dest="$3"
    local deb staging found

    # Tauri가 남겨 둔 deb 스테이징 디렉터리 (압축 해제 없이 사용)
    staging=$(find "${project_dir}/src-tauri/target/release/bundle/deb" -mindepth 1 -maxdepth 1 -type d ! -name "*.deb" 2>/dev/null | sort | tail -1)
    if [[ -n "${staging}" ]] && found=$(tauri_pick_usr_bin "${staging}" "${bin}"); then
        cp "${found}" "${dest}"
        chmod +x "${dest}" 2>/dev/null || true
        echo "${staging}"
        return 0
    fi

    deb=$(find "${project_dir}/src-tauri/target/release/bundle/deb" -name "*.deb" -type f 2>/dev/null | sort | tail -1)
    [[ -n "${deb}" ]] || return 1

    command -v dpkg-deb &>/dev/null || \
        DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends dpkg-dev >/dev/null 2>&1 || true
    command -v dpkg-deb &>/dev/null || return 1

    staging=$(mktemp -d)
    if ! dpkg-deb -x "${deb}" "${staging}" 2>/dev/null; then
        rm -rf "${staging}"
        return 1
    fi

    if found=$(tauri_pick_usr_bin "${staging}" "${bin}"); then
        cp "${found}" "${dest}"
        chmod +x "${dest}" 2>/dev/null || true
        rm -rf "${staging}"
        echo "${deb}"
        return 0
    fi

    rm -rf "${staging}"
    return 1
}

copy_tauri_binary() {
    local project="$1"
    local bin="$2"
    local src deb_src

    mkdir -p "${TOOLS_DIR}"
    if src=$(find_tauri_binary "${WSL_HOME}/${project}" "${bin}"); then
        cp "${src}" "${TOOLS_DIR}/${bin}"
        chmod +x "${TOOLS_DIR}/${bin}" 2>/dev/null || true
        ok "  ${bin} 빌드 완료 ($(du -sh "${TOOLS_DIR}/${bin}" | cut -f1)) ← ${src#${WSL_HOME}/}"
        return 0
    fi

    if deb_src=$(extract_tauri_binary_from_deb "${WSL_HOME}/${project}" "${bin}" "${TOOLS_DIR}/${bin}"); then
        ok "  ${bin} 빌드 완료 ($(du -sh "${TOOLS_DIR}/${bin}" | cut -f1)) ← deb:${deb_src#${WSL_HOME}/}"
        return 0
    fi

    warn "  ${bin} — 빌드 산출물 없음 (target/release·bundle·deb 확인)"
    find "${WSL_HOME}/${project}/src-tauri/target" -type f \( -name "${bin}" -o -name "*.AppImage" -o -name "*.deb" \) 2>/dev/null | \
        head -10 >> "${OUT_DIR}/gui-build.log" || true
    return 1
}

# ─── GUI 빌드 ────────────────────────────────────────────────────
build_gui_wsl() {
    step "GUI 빌드 (AegisCenter + Installer)"
    [[ $SKIP_GUI -eq 1 ]] && { ok "건너뜀"; return 0; }

    mkdir -p "${TOOLS_DIR}"

    if ! ensure_cargo; then
        warn "Cargo 없음 — GUI 빌드 건너뜀"
        return 0
    fi

    if ! ensure_nodejs; then
        warn "GUI 빌드 건너뜀 (CLI 도구만 ISO에 포함)"
        warn "GUI가 필요하면: sudo bash build/wsl-build.sh --setup"
        return 0
    fi

    if ! ensure_tauri_deps; then
        warn "GUI 빌드 건너뜀 (WebKitGTK 미설치)"
        return 0
    fi

    mkdir -p "${OUT_DIR}"
    : > "${OUT_DIR}/gui-build.log"

    if ! ensure_tauri_icons >> "${OUT_DIR}/gui-build.log" 2>&1; then
        warn "GUI 빌드 건너뜀 (Tauri 아이콘 준비 실패)"
        return 0
    fi

    # WSL에서 Tauri는 Xvfb 없이 빌드 가능 (DISPLAY 불필요)
    for project in gui installer; do
        log "${project} 빌드 중..."
        cd "${WSL_HOME}/${project}"
        if ! npm install --prefer-offline 2>/dev/null && ! npm install; then
            warn "${project}: npm install 실패 (건너뜀)"
            continue
        fi

        if tauri_webkit4_pkg_ready && ! tauri_webkit4_link_ready; then
            tauri_link_webkit41_libs_as_40 >> "${OUT_DIR}/gui-build.log" 2>&1 || true
        fi
        if ! tauri_webkit4_pkg_ready || ! tauri_webkit4_link_ready; then
            ensure_tauri_deps >> "${OUT_DIR}/gui-build.log" 2>&1 || true
        fi
        tauri_export_pkg_config
        tauri_log_webkit_status "${OUT_DIR}/gui-build.log"

        # 프로젝트 로컬 @tauri-apps/cli 사용 (cargo tauri 전역 설치 불필요)
        if DISPLAY="" env PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}" \
            PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1 PKG_CONFIG_ALLOW_SYSTEM_LIBS=1 \
            npx tauri build >> "${OUT_DIR}/gui-build.log" 2>&1; then
            :
        else
            warn "${project} 빌드 실패 (건너뜀)"
            tail -15 "${OUT_DIR}/gui-build.log" || true
            continue
        fi

        local bin="aegiscenter"
        [[ "$project" == "installer" ]] && bin="aegis-installer"
        copy_tauri_binary "${project}" "${bin}" || true
    done
}

# ─── chroot 파일 설치 ────────────────────────────────────────────
populate_chroot_wsl() {
    step "chroot 파일시스템 구성"

    wsl_mount_chroot "${CHROOT_DIR}"

    # Live 부팅 패키지/설정 (기존 chroot 업데이트 시에도 적용)
    chroot "${CHROOT_DIR}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get install -y --no-install-recommends \
        live-config-systemd casper 2>/dev/null || \
        warn "live-config-systemd 설치 실패 — apt update 후 재시도"
    mkdir -p "${CHROOT_DIR}/etc/live"
    cat > "${CHROOT_DIR}/etc/live/config.conf" <<'EOF'
LIVE_HOSTNAME="aegisrescue"
LIVE_USERNAME="aegis"
LIVE_CONFIG_CMDLINE="false"
EOF

    # Aegis 도구
    mkdir -p "${CHROOT_DIR}/opt/aegis/bin"
    if [[ -d "${TOOLS_DIR}" ]]; then
        cp -v "${TOOLS_DIR}/"* "${CHROOT_DIR}/opt/aegis/bin/" 2>/dev/null | \
            grep -v "^$" | head -20 || true
        for gui_bin in aegiscenter aegis-installer; do
            [[ -f "${CHROOT_DIR}/opt/aegis/bin/${gui_bin}" ]] || \
                warn "chroot에 ${gui_bin} 없음 — build/tools 확인"
        done
    else
        warn "build/tools 없음"
    fi

    # PATH
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
    mkdir -p "${CHROOT_DIR}/etc/aegis" "${CHROOT_DIR}/usr/share/aegis"
    cp "${WSL_HOME}/packaging/policy/policy.toml" \
       "${CHROOT_DIR}/etc/aegis/policy.toml" 2>/dev/null || true
    cp "${WSL_HOME}/packaging/policy/policy.toml" \
       "${CHROOT_DIR}/usr/share/aegis/policy.toml" 2>/dev/null || true
    cp "${WSL_HOME}/security/nftables/airgap.nft" \
       "${CHROOT_DIR}/etc/nftables.conf" 2>/dev/null || true
    mkdir -p "${CHROOT_DIR}/etc/apparmor.d"
    cp "${WSL_HOME}/security/apparmor/"*.apparmor \
       "${CHROOT_DIR}/etc/apparmor.d/" 2>/dev/null || true

    # 데이터 파일
    mkdir -p "${CHROOT_DIR}/var/lib/aegis/"{yara-rules,signatures,models}
    cp -r "${WSL_HOME}/data/yara-rules/"* \
          "${CHROOT_DIR}/var/lib/aegis/yara-rules/" 2>/dev/null || true
    cp -r "${WSL_HOME}/data/signatures/"* \
          "${CHROOT_DIR}/var/lib/aegis/signatures/" 2>/dev/null || true

    # 부팅 스플래시(Plymouth) + 데스크톱 로고
    # shellcheck source=stage-boot-logos.sh
    source "${WSL_HOME}/build/stage-boot-logos.sh"
    if install_chroot_boot_logos "${WSL_HOME}" "${CHROOT_DIR}"; then
        ok "chroot 부팅 로고/Plymouth 테마 적용"
    else
        warn "chroot 부팅 로고 없음 — assets/aegisrescue-boot-logo-black.png 확인"
    fi

    # systemd 서비스
    mkdir -p "${CHROOT_DIR}/etc/systemd/system"
    for f in "${WSL_HOME}/packaging/systemd/"*; do
        cp "$f" "${CHROOT_DIR}/etc/systemd/system/" 2>/dev/null || true
    done

    # initramfs 훅
    mkdir -p "${CHROOT_DIR}/etc/initramfs-tools/hooks"
    mkdir -p "${CHROOT_DIR}/etc/initramfs-tools/scripts/live-bottom"
    cp "${WSL_HOME}/build/initramfs/hooks/aegis" \
       "${CHROOT_DIR}/etc/initramfs-tools/hooks/aegis" 2>/dev/null || true
    cp "${WSL_HOME}/build/initramfs/scripts/live-bottom/aegis-mode" \
       "${CHROOT_DIR}/etc/initramfs-tools/scripts/live-bottom/aegis-mode" 2>/dev/null || true
    chmod +x \
        "${CHROOT_DIR}/etc/initramfs-tools/hooks/aegis" \
        "${CHROOT_DIR}/etc/initramfs-tools/scripts/live-bottom/aegis-mode" \
        2>/dev/null || true

    # 서비스 활성화
    # shellcheck source=configure-live-console.sh
    source "${WSL_HOME}/build/configure-live-console.sh"
    configure_live_console "${CHROOT_DIR}"

    # shellcheck source=configure-live-gui.sh
    source "${WSL_HOME}/build/configure-live-gui.sh"
    install_live_gui_scripts "${CHROOT_DIR}" "${WSL_HOME}"
    configure_live_gui "${CHROOT_DIR}" "${WSL_HOME}"

    chroot "${CHROOT_DIR}" systemctl enable aegisd.service 2>/dev/null || true
    chroot "${CHROOT_DIR}" systemctl enable aegis-preflight.service 2>/dev/null || true
    chroot "${CHROOT_DIR}" systemctl enable aegis-update.timer 2>/dev/null || true

    # initramfs 재생성
    log "initramfs 재생성 중..."
    chroot "${CHROOT_DIR}" update-initramfs -u -k all 2>/dev/null || \
        warn "initramfs 재생성 실패 (커널이 없을 수 있음)"

    wsl_umount_chroot "${CHROOT_DIR}"
    ok "chroot 구성 완료"
}

# ─── squashfs + ISO ──────────────────────────────────────────────
build_iso_wsl() {
    step "squashfs 및 ISO 생성"

    mkdir -p "${OUT_DIR}" "${ISO_STAGE}"/{boot/grub,boot/grub/fonts,EFI/BOOT,live,isolinux}

    # squashfs
    log "squashfs 생성 중 (xz 압축, ${JOBS} 코어)..."
    log "예상 시간: 5-20분"
    mksquashfs "${CHROOT_DIR}" "${ISO_STAGE}/live/filesystem.squashfs" \
        -comp xz \
        -Xbcj x86 \
        -b 1M \
        -processors "${JOBS}" \
        -wildcards \
        -ef "${WSL_HOME}/build/squashfs-excludes.txt" \
        -noappend \
        -progress \
        2>&1 | grep -E "^(Parallel|Deflating|[0-9]+%|Created)" | tail -5 || true

    local sq_size
    sq_size=$(du -sh "${ISO_STAGE}/live/filesystem.squashfs" | cut -f1)
    ok "squashfs: ${sq_size}"
    du -sx --block-size=1 "${CHROOT_DIR}" 2>/dev/null | cut -f1 \
        > "${ISO_STAGE}/live/filesystem.size" || true

    du -s --block-size=1 "${CHROOT_DIR}" | cut -f1 \
        > "${ISO_STAGE}/live/filesystem.size"

    # 커널 / initramfs 복사
    local vmlinuz initrd
    vmlinuz=$(ls "${CHROOT_DIR}/boot/vmlinuz-"* 2>/dev/null | sort -V | tail -1 || echo "")
    initrd=$(ls "${CHROOT_DIR}/boot/initrd.img-"* 2>/dev/null | sort -V | tail -1 || echo "")

    if [[ -f "$vmlinuz" ]]; then
        cp "$vmlinuz" "${ISO_STAGE}/boot/vmlinuz"
        ok "커널: $(basename "$vmlinuz")"
    else
        warn "커널 없음 — debootstrap 단계 확인 필요"
        # 더미 파일 (테스트용)
        dd if=/dev/zero bs=1M count=1 of="${ISO_STAGE}/boot/vmlinuz" 2>/dev/null
    fi

    [[ -f "$initrd" ]] && cp "$initrd" "${ISO_STAGE}/boot/initramfs.img"

    # GRUB 설정
    cp "${WSL_HOME}/build/grub/grub.cfg" "${ISO_STAGE}/boot/grub/grub.cfg"
    mkdir -p "${ISO_STAGE}/boot/grub/themes/aegis"
    cp "${WSL_HOME}/build/grub/theme.txt" \
       "${ISO_STAGE}/boot/grub/themes/aegis/theme.txt" 2>/dev/null || \
       cp "${WSL_HOME}/build/grub/theme.txt" \
          "${ISO_STAGE}/boot/grub/themes/aegis/theme.txt" 2>/dev/null || true

    # 부팅 로고/배경 (GRUB + ISOLINUX + EFI embed)
    # shellcheck source=stage-boot-logos.sh
    source "${WSL_HOME}/build/stage-boot-logos.sh"
    if stage_iso_boot_logos "${WSL_HOME}" "${ISO_STAGE}"; then
        ok "부팅 로고 적용: aegisrescue-boot-logo-black.png"
    else
        warn "부팅 로고 파일을 찾지 못했습니다. AEGIS_BOOT_LOGO=/path/logo.png 지정 가능"
    fi

    # 폰트
    local font
    font=$(find /usr/share/grub -name "unicode.pf2" 2>/dev/null | head -1)
    [[ -n "$font" ]] && cp "$font" "${ISO_STAGE}/boot/grub/fonts/"

    # SYSLINUX (BIOS)
    setup_syslinux_wsl

    # EFI 이미지
    create_efi_wsl

    # ISO 생성
    log "xorriso로 ISO 생성 중..."
    xorriso -as mkisofs \
        -iso-level 3 \
        -full-iso9660-filenames \
        -volid "AEGISRESCUE_BETA_${VERSION//./_}" \
        -appid "${ISO_LABEL}" \
        -publisher "AegisRescue Project" \
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
        2>&1 | grep -v "^$" | tail -15

    # 체크섬
    cd "${OUT_DIR}"
    sha256sum "$(basename "${ISO_NAME}")" > "$(basename "${ISO_NAME}").sha256"
    md5sum    "$(basename "${ISO_NAME}")" > "$(basename "${ISO_NAME}").md5"

    # Windows에 복사
    if [[ -d "${WIN_SRC}/build/out" ]] || mkdir -p "${WIN_SRC}/build/out" 2>/dev/null; then
        log "Windows 출력 폴더로 복사 중..."
        cp "${ISO_NAME}"* "${WIN_SRC}/build/out/" 2>/dev/null && \
            ok "Windows로 복사 완료: $(win_display_path "${WIN_SRC}/build/out/")"
    fi

    local iso_size
    iso_size=$(du -sh "${ISO_NAME}" | cut -f1)

    echo ""
    echo -e "${G}${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
    echo -e "${G}${B}  ✓ AegisRescue Beta ISO 빌드 완료!${N}"
    echo -e "${G}${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
    echo -e "  ISO: ${C}${ISO_NAME}${N}"
    echo -e "  크기: ${iso_size}"
    echo -e "  SHA256: $(head -1 "${ISO_NAME}.sha256" | cut -d' ' -f1)"
    echo -e "  Windows 경로: ${Y}$(win_display_path "${WIN_SRC}/build/out/$(basename "${ISO_NAME}")")${N}"
    echo ""
    echo -e "  USB 기록 (WSL):   ${Y}sudo dd if=${ISO_NAME} of=/dev/sdX bs=4M status=progress${N}"
    echo -e "  USB 기록 (Win):   ${Y}Rufus 또는 balenaEtcher 사용${N}"
    echo -e "  QEMU 테스트:      ${Y}make -C ${WSL_HOME}/build test-qemu${N}"
    echo -e "${G}${B}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
}

setup_syslinux_wsl() {
    # shellcheck source=stage-isolinux.sh
    source "${WSL_HOME}/build/stage-isolinux.sh"
    # shellcheck source=configure-live-console.sh
    source "${WSL_HOME}/build/configure-live-console.sh"

    local isolinux_rc=0
    stage_isolinux_files "${ISO_STAGE}" || isolinux_rc=$?
    if [[ ${isolinux_rc} -eq 0 ]]; then
        ok "SYSLINUX 파일 복사됨 (isolinux.bin + ldlinux.c32)"
    elif [[ ${isolinux_rc} -eq 2 ]]; then
        err "ldlinux.c32 없음 — sudo apt install syslinux-common 후 ISO 재빌드"
    else
        warn "SYSLINUX 없음 — BIOS 부팅 제한됨 (apt install isolinux syslinux-common)"
        dd if=/dev/zero bs=512 count=1  of="${ISO_STAGE}/isolinux/isohdpfx.bin" 2>/dev/null
        dd if=/dev/zero bs=512 count=4  of="${ISO_STAGE}/isolinux/isolinux.bin"  2>/dev/null
        touch "${ISO_STAGE}/isolinux/boot.cat"
    fi

    [[ -f "${ISO_STAGE}/isolinux/boot.cat" ]] || touch "${ISO_STAGE}/isolinux/boot.cat"

    cat > "${ISO_STAGE}/isolinux/isolinux.cfg" <<EOF
DEFAULT vesamenu.c32
TIMEOUT 100
MENU TITLE AegisRescue Beta ${VERSION}
MENU BACKGROUND splash.png
EOF
    cat "${WSL_HOME}/build/isolinux/vesamenu-colors.cfg" >> "${ISO_STAGE}/isolinux/isolinux.cfg"
    cat >> "${ISO_STAGE}/isolinux/isolinux.cfg" <<EOF
LABEL live
  MENU LABEL AegisRescue Live (GUI + Installer)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_GUI}
LABEL cli
  MENU LABEL CLI Mode (Text Console)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_CLI}
LABEL forensic
  MENU LABEL Forensic Mode (CLI)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_FORENSIC}
LABEL airgap
  MENU LABEL Air-Gap Mode (CLI)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_AIRGAP}
LABEL debug
  MENU LABEL Debug (verbose)
  KERNEL /boot/vmlinuz
  APPEND initrd=/boot/initramfs.img ${LIVE_KERNEL_COMMON} debug verbose systemd.log_level=debug
EOF
}

create_efi_wsl() {
    local efi_img="${ISO_STAGE}/EFI/BOOT/efiboot.img"
    # shellcheck source=grub-efi-standalone.sh
    source "${WSL_HOME}/build/grub-efi-standalone.sh"

    log "GRUB EFI 스탠드얼론 이미지 생성 중 (로고/폰트 포함)..."
    if create_grub_efi_standalone "${ISO_STAGE}" "${ISO_STAGE}/boot/grub/grub.cfg" "${efi_img}"; then
        ok "EFI 이미지 생성됨"
    else
        warn "EFI 이미지 생성 실패 — grub-mkstandalone 확인"
    fi
}

# ─── QEMU 테스트 ─────────────────────────────────────────────────
run_qemu_test() {
    step "QEMU 테스트"
    command -v qemu-system-x86_64 &>/dev/null || {
        warn "QEMU 없음: sudo apt install qemu-system-x86 ovmf"
        return 0
    }

    local ovmf
    ovmf=$(find /usr/share -name "OVMF.fd" 2>/dev/null | head -1)

    log "ISO: ${ISO_NAME}"
    log "모드: ${ovmf:+UEFI}${ovmf:-BIOS}"

    if [[ -n "$ovmf" ]]; then
        qemu-system-x86_64 \
            -m 2G -smp 2 \
            -bios "$ovmf" \
            -cdrom "${ISO_NAME}" \
            -boot d \
            -vga virtio \
            -nographic \
            -serial mon:stdio \
            -no-reboot \
            -enable-kvm 2>/dev/null || \
        qemu-system-x86_64 \
            -m 2G -smp 2 \
            -bios "$ovmf" \
            -cdrom "${ISO_NAME}" \
            -vga std \
            -nographic 2>/dev/null || true
    else
        qemu-system-x86_64 \
            -m 2G -smp 2 \
            -cdrom "${ISO_NAME}" \
            -boot d \
            -vga std \
            -nographic \
            -serial mon:stdio \
            -no-reboot 2>/dev/null || true
    fi
}

# ─── 메인 ────────────────────────────────────────────────────────
main() {
    echo -e "${B}"
    echo "  ╔═══════════════════════════════════════════════════════╗"
    echo "  ║       AegisRescue — WSL2 ISO 빌드 시스템             ║"
    echo "  ║  버전: ${VERSION}  아키텍처: ${ARCH}  코어: ${JOBS}             ║"
    echo "  ╚═══════════════════════════════════════════════════════╝"
    echo -e "${N}"

    # --setup는 root 필요하지만 먼저 처리
    if [[ $DO_SETUP -eq 1 ]]; then
        [[ $EUID -eq 0 ]] || err "setup은 root 권한 필요: sudo bash build/wsl-build.sh --setup"
        setup_wsl
        exit 0
    fi

    # --copy는 root 불필요
    if [[ $DO_COPY -eq 1 ]]; then
        copy_source
        exit 0
    fi

    detect_wsl
    [[ $EUID -eq 0 ]] || err "ISO 빌드는 root 권한 필요: sudo bash build/wsl-build.sh"

    mkdir -p "${OUT_DIR}"
    trap 'wsl_umount_chroot "${CHROOT_DIR}" 2>/dev/null || true' EXIT

    local start=$SECONDS

    # 소스가 WSL 홈에 없으면 자동 복사
    if [[ ! -f "${WSL_HOME}/Cargo.toml" ]]; then
        log "WSL 홈에 소스 없음 — 자동 복사 시도..."
        copy_source
    fi
    cd "${WSL_HOME}"

    [[ $ISO_ONLY -eq 0 ]] && build_base_wsl
    [[ $ISO_ONLY -eq 0 ]] && build_rust_wsl
    [[ $ISO_ONLY -eq 0 ]] && build_gui_wsl
    [[ $ISO_ONLY -eq 0 ]] && populate_chroot_wsl
    build_iso_wsl

    local elapsed=$(( SECONDS - start ))
    echo -e "\n⏱ 총 빌드 시간: $((elapsed/60))분 $((elapsed%60))초"

    [[ $QEMU_TEST -eq 1 ]] && run_qemu_test
}

main "$@"
