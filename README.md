# AegisRescue Linux

<p align="center">
  <img src="docs/assets/aegisrescue-logo.svg" alt="AegisRescue OS logo" width="720">
</p>

> 악성코드 감염 시스템 복구 및 디지털 포렌식 전용 Live Linux 운영체제

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.79+-orange.svg)](https://www.rust-lang.org)
[![Architecture](https://img.shields.io/badge/Arch-x86__64%20%7C%20ARM64-green.svg)]()

---

## 개요

AegisRescue는 Ubuntu의 사용성, Kali Linux의 보안 도구 생태계,
SystemRescue의 복구 능력, Clonezilla의 디스크 이미징 기능을 결합한
**Rust 우선(Rust-first) 보안 복구 전용 Live OS**입니다.

### 핵심 기능

| 기능 | 설명 |
|------|------|
| 멀티엔진 악성코드 탐지 | ClamAV + YARA + 휴리스틱 + AI 분류 |
| 랜섬웨어 복구 | 알려진 복호화기 + VSS + 파일 카빙 |
| 안전 마운트 | 감염 디스크 RO+noexec 강제 마운트 |
| 부트 복구 | MBR/GPT/EFI/BCD/GRUB 복원 |
| 디지털 포렌식 | 해시 체인, 타임라인, 아티팩트 수집 |
| 디스크 이미징 | 손상 복사(ddrescue) + 압축 이미지 |
| AI 분석 | 오프라인 정적 분류기 (ONNX/tract) |
| 네트워크 격리 | Air-Gap 모드 기본값 |

### 최소 사양

- RAM: 2GB (4GB 권장)
- CPU: x86_64 또는 ARM64 (aarch64), 듀얼코어 이상
- USB: 4GB 이상 (8GB 권장, toram 모드 시 8GB+)

---

## 빌드 요구사항

```bash
# Rust 툴체인
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-musl

# 시스템 패키지 (Debian/Ubuntu 빌드 호스트)
sudo apt install -y \
    build-essential gcc-aarch64-linux-gnu \
    squashfs-tools mksquashfs \
    grub-pc-bin grub-efi-amd64-bin \
    xorriso dosfstools \
    libssl-dev pkg-config

# Node.js (GUI 빌드용)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
```

## 빠른 빌드

```bash
# 1) Rust 코어 빌드
cargo build --release --workspace

# 2) GUI 빌드 (Tauri)
cd gui && npm install && cargo tauri build

# 3) Live ISO 생성
make -C build iso ARCH=x86_64

# 4) USB에 기록
sudo dd if=build/out/aegisrescue-x86_64.iso of=/dev/sdX \
    bs=4M status=progress oflag=sync
```

---

## 프로젝트 구조

```
aegisrescue/
├── core/               # Rust 워크스페이스 (모든 핵심 도구)
│   ├── aegis-common/   # 공용 라이브러리
│   ├── aegisd/         # 시스템 데몬
│   ├── aegis-mount/    # 안전 마운트 관리자
│   ├── aegis-scan/     # 악성코드 탐지 엔진
│   ├── aegis-recover/  # 랜섬웨어/파일 복구
│   ├── aegis-boot/     # 부트 레코드 복구
│   ├── aegis-forensic/ # 디지털 포렌식
│   ├── aegis-image/    # 디스크 이미징/클론
│   ├── aegis-ai/       # AI 악성코드 분류
│   ├── aegis-sandbox/  # 샌드박스 런처
│   └── aegis-cli/      # 통합 CLI
├── gui/                # AegisCenter (Tauri + Svelte)
├── kernel/             # 커널 설정/패치
├── security/           # AppArmor, seccomp, nftables
├── data/               # YARA룰, 시그니처, AI 모델
├── build/              # ISO 빌드 스크립트
├── packaging/          # .deb 패키징
├── docs/               # 설계/개발 문서
└── tests/              # 통합 테스트
```

---

## 라이선스

- 코어 OS 및 도구: **GPLv3**
- Rust 라이브러리 컴포넌트: **Apache-2.0 OR MIT**

## 기여

[CONTRIBUTING.md](docs/CONTRIBUTING.md) 참조
