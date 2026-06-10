# AegisRescue 빠른 시작 가이드

## 1. 개발 빌드 (Linux 빌드 호스트)

```bash
# 의존성 설치 (Ubuntu/Debian)
sudo apt install -y \
    build-essential gcc libc6-dev \
    libssl-dev pkg-config \
    squashfs-tools xorriso \
    grub-pc-bin grub-efi-amd64-bin \
    nodejs npm

# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 빌드
git clone https://github.com/aegisrescue/aegisrescue
cd aegisrescue
cargo build --workspace

# CLI 테스트
cargo run --bin aegis -- --help
cargo run --bin aegis-scan -- --help
cargo run --bin aegisd -- --help
```

## 2. 핵심 도구 사용법

### aegisd (데몬) 시작
```bash
sudo cargo run --bin aegisd -- --foreground --log-level debug
```

### 악성코드 스캔
```bash
# 빠른 스캔
cargo run --bin aegis-scan -- --quick /path/to/target

# 전체 스캔 + JSON 결과
cargo run --bin aegis-scan -- --full --json /path/to/target

# 탐지 시 자동 격리
cargo run --bin aegis-scan -- --full --quarantine /path/to/target
```

### 랜섬웨어 복구
```bash
# 패밀리 식별
cargo run --bin aegis-recover -- identify /mnt/infected

# 자동 복구 (모든 방법 시도)
cargo run --bin aegis-recover -- auto /mnt/infected \
    --output /mnt/recovered
```

### 부트 복구
```bash
# 부트 레코드 분석
cargo run --bin aegis-boot -- analyze /dev/sda

# 부트 레코드 백업 후 복구
cargo run --bin aegis-boot -- repair /dev/sda --windows
```

### 안전 마운트
```bash
# 읽기 전용 마운트
sudo cargo run --bin aegis-mount -- mount /dev/sdb1

# 마운트 목록
cargo run --bin aegis-mount -- list
```

### AI 분석
```bash
cargo run --bin aegis-ai -- /path/to/suspicious_file.exe
cargo run --bin aegis-ai -- --json /path/to/file.dll
```

### 포렌식
```bash
# 증거 수집
cargo run --bin aegis-forensic -- capture /mnt/target \
    --output /mnt/evidence

# 타임라인 생성
cargo run --bin aegis-forensic -- timeline /mnt/target
```

### 샌드박스 분석
```bash
# L0: 정적 분석만
cargo run --bin aegis-sandbox -- run /path/to/sample --level 0

# L1: 네임스페이스 격리 (Linux 전용)
sudo cargo run --bin aegis-sandbox -- run /path/to/sample --level 1
```

## 3. 단위 테스트

```bash
# 전체 테스트
cargo test --workspace

# 특정 크레이트
cargo test -p aegis-common
cargo test -p aegis-ai
```

## 4. ISO 빌드 (완전한 Live OS)

```bash
# x86_64 ISO 빌드
make -C build iso ARCH=x86_64 VERSION=0.1.0

# USB에 기록
make -C build write-usb USB=/dev/sdc
```
