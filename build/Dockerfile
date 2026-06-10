# AegisRescue ISO 빌드 컨테이너
# 사용법:
#   docker build -t aegisrescue-builder ./build/
#   docker run --privileged -v $(pwd):/workspace aegisrescue-builder

FROM ubuntu:24.04

LABEL maintainer="AegisRescue Project"
LABEL description="AegisRescue ISO build environment"

ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=en_US.UTF-8
ENV RUST_VERSION=stable

# ─── 시스템 패키지 ────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    # 빌드 도구
    build-essential git curl wget ca-certificates \
    pkg-config libssl-dev \
    # ISO / FS 도구
    xorriso mksquashfs squashfs-tools \
    grub-pc-bin grub-efi-amd64-bin grub-efi-amd64-signed \
    grub2-common grub-common \
    isolinux syslinux syslinux-common syslinux-efi \
    dosfstools mtools \
    # 기반 시스템 구성 도구
    debootstrap systemd-container \
    # 압축 도구
    xz-utils zstd lz4 gzip bzip2 \
    # 유틸리티
    rsync pv sha256deep \
    locales sudo \
    # musl (정적 바이너리용)
    musl-tools musl-dev \
    # Tauri 의존성
    libwebkit2gtk-4.0-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev \
    # Node.js 설치용
    gpg \
    && rm -rf /var/lib/apt/lists/*

# ─── Rust 설치 ────────────────────────────────────────────────
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_VERSION} --no-modify-path && \
    /root/.cargo/bin/rustup target add \
        x86_64-unknown-linux-musl \
        aarch64-unknown-linux-musl 2>/dev/null || true

ENV PATH="/root/.cargo/bin:${PATH}"

# ─── Node.js 설치 ─────────────────────────────────────────────
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - && \
    apt-get install -y nodejs && \
    node --version && npm --version

# ─── 로케일 설정 ──────────────────────────────────────────────
RUN locale-gen en_US.UTF-8 ko_KR.UTF-8 ja_JP.UTF-8 && \
    update-locale LANG=en_US.UTF-8

WORKDIR /workspace

# 빌드 캐시를 위한 더미 레이어
RUN mkdir -p /workspace/build/out

ENTRYPOINT ["bash", "build/build.sh"]
CMD ["--help"]
