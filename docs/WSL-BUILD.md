# AegisRescue — WSL2 빌드 가이드

## 개요

Windows에서 WSL2(Windows Subsystem for Linux 2)를 사용하여 AegisRescue ISO를 빌드합니다.

---

## 사전 조건

| 항목 | 요구사항 |
|------|----------|
| OS | Windows 10 21H2+ 또는 Windows 11 |
| WSL | WSL2 (WSL1 불가) |
| Linux | Ubuntu 22.04 LTS 또는 24.04 LTS |
| RAM | 호스트 8GB+ (WSL에 4GB 이상 할당) |
| 디스크 | 여유 공간 20GB+ (chroot + squashfs + ISO) |
| 인터넷 | debootstrap 및 패키지 다운로드 필요 |

---

## 방법 1 — 자동 설정 스크립트 (권장)

```powershell
# 1. PowerShell을 관리자로 실행
# 2. 실행 정책 임시 허용
Set-ExecutionPolicy -Scope Process Bypass

# 3. 자동 설정 실행 (WSL2 + Ubuntu + 패키지 일괄 설치)
cd F:\ofntkd\aegisrescue
.\build\wsl-setup.ps1

# 4. WSL이 재시작된 후 빌드 실행
wsl -u root -- bash /mnt/f/ofntkd/aegisrescue/build/wsl-build.sh
```

---

## 방법 2 — 단계별 수동 설정

### 2-1. WSL2 활성화

```powershell
# 관리자 PowerShell
wsl --install
wsl --set-default-version 2
# 재부팅 후
wsl --install Ubuntu-24.04
```

### 2-2. WSL 메모리/CPU 설정 (`%USERPROFILE%\.wslconfig`)

```ini
[wsl2]
memory=6GB
processors=4
swap=2GB
localhostForwarding=true
nestedVirtualization=true
```

변경 후 적용:
```powershell
wsl --shutdown
```

### 2-3. Ubuntu에서 의존성 설치

```bash
# WSL Ubuntu 터미널
sudo apt-get update
sudo apt-get install -y \
    debootstrap xorriso squashfs-tools \
    grub-pc-bin grub-efi-amd64-bin grub-efi-amd64-signed \
    grub2-common isolinux syslinux syslinux-common \
    dosfstools mtools xz-utils zstd lz4 \
    rsync curl wget ca-certificates git \
    build-essential pkg-config libssl-dev \
    musl-tools musl-dev \
    libwebkit2gtk-4.0-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev \
    qemu-system-x86 ovmf

# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable
source ~/.cargo/env
rustup target add x86_64-unknown-linux-musl

# Node.js 설치
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo bash -
sudo apt-get install -y nodejs
```

---

## 방법 3 — 빠른 빌드 런처 (PowerShell)

```powershell
# 기본 빌드
.\build\wsl-quick.ps1

# 옵션 지정
.\build\wsl-quick.ps1 -Arch x86_64 -Version 0.1.0 -Jobs 8

# Rust/GUI 빌드 건너뜀 (재빌드 시)
.\build\wsl-quick.ps1 -SkipRust -SkipGui

# ISO만 재생성 (chroot 재사용)
.\build\wsl-quick.ps1 -IsoOnly
```

---

## 전체 빌드 순서

```
F:\ofntkd\aegisrescue\
  └── build\
       ├── wsl-setup.ps1     ← 1. WSL 환경 설정 (관리자 PowerShell)
       ├── wsl-quick.ps1     ← 2. 빌드 실행 런처
       └── wsl-build.sh      ← 3. WSL 내부 빌드 스크립트 (sudo 필요)
```

### 전체 흐름 (약 30-60분)

```
wsl-build.sh
  │
  ├── [1] debootstrap        Ubuntu 24.04 minimal 설치     (~10분)
  │        └── 커널, live-boot, 기본 패키지
  │
  ├── [2] Rust 빌드          musl 정적 바이너리 생성        (~15분)
  │        └── aegisd, aegis-scan, aegis-recover, ...
  │
  ├── [3] GUI 빌드           Tauri + Svelte                 (~10분)
  │        └── aegiscenter, aegis-installer
  │
  ├── [4] chroot 구성        도구/설정/서비스 설치          (~3분)
  │        └── /opt/aegis/bin, /etc/aegis, systemd 서비스
  │
  ├── [5] mksquashfs         xz 압축 (1MB 블록)             (~10분)
  │        └── build/iso-stage/live/filesystem.squashfs
  │
  └── [6] xorriso            하이브리드 ISO 생성            (~2분)
           └── build/out/aegisrescue-0.1.0-x86_64.iso
```

---

## WSL 주요 제한 사항 및 해결책

| 제한 | 원인 | 해결책 |
|------|------|--------|
| `/mnt/f/` 느린 빌드 | Windows FS 접근 overhead | WSL 홈(`~/`)에서 빌드 (스크립트 자동 처리) |
| `mount --bind` 실패 | WSL 커널 제한 | `mount -t proc/sysfs/devtmpfs` 폴백 사용 |
| `loop` 디바이스 없음 | WSL 커널 모듈 | `modprobe loop` 또는 QEMU 테스트 시 WSLg 사용 |
| GUI 렌더링 | X 서버 없음 | Tauri 빌드는 headless로 가능, QEMU `-nographic` |
| `systemd` 미작동 | 기본 비활성 | `/etc/wsl.conf`에 `systemd=true` 추가 후 재시작 |

---

## 빌드 옵션

```bash
# WSL 터미널에서
sudo bash ~/aegisrescue/build/wsl-build.sh [옵션]

옵션:
  --setup          WSL 환경 초기 설정 (첫 실행 시)
  --copy           Windows 소스를 WSL 홈으로 복사
  --arch ARCH      x86_64 | aarch64 (기본: x86_64)
  --ver VERSION    버전 문자열 (기본: 0.1.0)
  --jobs N         병렬 빌드 수 (기본: nproc)
  --skip-rust      Rust 빌드 건너뜀
  --skip-gui       GUI 빌드 건너뜀
  --skip-base      debootstrap 건너뜀 (기존 chroot 재사용)
  --iso-only       squashfs + ISO만 재생성 (가장 빠름)
  --qemu-test      빌드 후 QEMU 자동 테스트
```

### 자주 쓰는 조합

```bash
# 초기 전체 빌드
sudo bash build/wsl-build.sh

# Rust 코드만 바꿨을 때 (debootstrap 재사용)
sudo bash build/wsl-build.sh --skip-base --skip-gui

# ISO 레이아웃만 바꿨을 때 (가장 빠름, 2-3분)
sudo bash build/wsl-build.sh --iso-only

# ARM64 빌드
sudo bash build/wsl-build.sh --arch aarch64

# QEMU로 바로 테스트
sudo bash build/wsl-build.sh --iso-only --qemu-test
```

---

## 출력 파일

```
~/aegisrescue/build/out/
  aegisrescue-0.1.0-x86_64.iso        ← 하이브리드 ISO (BIOS + UEFI)
  aegisrescue-0.1.0-x86_64.iso.sha256 ← SHA256 체크섬
  aegisrescue-0.1.0-x86_64.iso.md5    ← MD5 체크섬
  rust-build.log                       ← Rust 빌드 로그
  gui-build.log                        ← GUI 빌드 로그

# Windows 경로로도 자동 복사
F:\ofntkd\aegisrescue\build\out\
  aegisrescue-0.1.0-x86_64.iso
```

---

## QEMU 테스트 (WSL에서)

```bash
# BIOS 모드
qemu-system-x86_64 \
    -m 2G -smp 2 \
    -cdrom ~/aegisrescue/build/out/aegisrescue-0.1.0-x86_64.iso \
    -boot d -vga std -nographic

# UEFI 모드
OVMF=$(find /usr/share -name "OVMF.fd" | head -1)
qemu-system-x86_64 \
    -m 2G -smp 2 \
    -bios "$OVMF" \
    -cdrom ~/aegisrescue/build/out/aegisrescue-0.1.0-x86_64.iso \
    -vga virtio -nographic
```

### WSLg를 통한 GUI 테스트 (Windows 11)

```bash
# WSLg가 활성화된 경우 X 윈도우 자동 지원
qemu-system-x86_64 \
    -m 2G -smp 2 \
    -cdrom ~/aegisrescue/build/out/aegisrescue-0.1.0-x86_64.iso \
    -boot d -vga virtio -display gtk
```

---

## USB 기록

### Windows (Rufus 사용 — 권장)
1. [Rufus](https://rufus.ie) 다운로드
2. `aegisrescue-0.1.0-x86_64.iso` 선택
3. 파티션: `GPT`, 대상: `UEFI (비 CSM)`
4. 시작 클릭

### Windows (balenaEtcher)
```
파일: F:\ofntkd\aegisrescue\build\out\aegisrescue-0.1.0-x86_64.iso
```

### WSL (dd)
```bash
# 주의: 모든 데이터 삭제됨
sudo dd if=~/aegisrescue/build/out/aegisrescue-0.1.0-x86_64.iso \
        of=/dev/sdX bs=4M status=progress oflag=sync
```

---

## 문제 해결

### `debootstrap` 실패
```bash
# 미러 변경 후 재시도
sudo bash build/wsl-build.sh --skip-base=0
# 또는 한국 미러 사용
export DEBOOTSTRAP_MIRROR="http://kr.archive.ubuntu.com/ubuntu/"
```

### Rust 빌드 오류 (`musl-gcc` 없음)
```bash
sudo apt-get install -y musl-tools
rustup target add x86_64-unknown-linux-musl
```

### `xorriso` 실패 (`isohdpfx.bin` 없음)
```bash
sudo apt-get install -y isolinux syslinux syslinux-common
```

### WSL 메모리 부족 (OOM)
`.wslconfig`에서 메모리 증가:
```ini
[wsl2]
memory=8GB
```
그 후: `wsl --shutdown`

### `/mnt/f/` 권한 오류
Windows 파일시스템은 Linux 권한 모델이 제한됩니다.
빌드 스크립트가 자동으로 `~/aegisrescue/`로 복사합니다.
