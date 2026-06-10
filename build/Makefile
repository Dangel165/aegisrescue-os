# =============================================================================
# AegisRescue 빌드 시스템 (GNU Make)
# =============================================================================
# 사용법:
#   make                     — 전체 ISO 빌드 (기본)
#   make iso                 — ISO 생성
#   make rust                — Rust 도구만 빌드
#   make gui                 — GUI만 빌드
#   make squashfs            — squashfs만 재생성
#   make test-qemu           — QEMU로 ISO 부팅 테스트
#   make test-qemu-uefi      — QEMU UEFI 모드 테스트
#   make write-usb USB=/dev/sdX  — USB에 기록
#   make docker-build        — Docker 컨테이너로 빌드
#   make clean               — 빌드 아티팩트 정리
#   make distclean           — chroot 포함 전체 삭제
#
# 환경 변수:
#   ARCH=x86_64|aarch64      대상 아키텍처 (기본: x86_64)
#   VERSION=0.1.0            버전 (기본: 0.1.0)
#   JOBS=N                   병렬 빌드 수 (기본: nproc)
# =============================================================================

SHELL        := /bin/bash
.SHELLFLAGS  := -euo pipefail -c

ARCH         ?= x86_64
VERSION      ?= 0.1.0
JOBS         ?= $(shell nproc 2>/dev/null || echo 4)

PROJECT_ROOT := $(abspath $(CURDIR)/..)
BUILD_DIR    := $(PROJECT_ROOT)/build
OUT_DIR      := $(BUILD_DIR)/out
CHROOT_DIR   := $(BUILD_DIR)/chroot
ISO_STAGE    := $(BUILD_DIR)/iso-stage
TOOLS_DIR    := $(BUILD_DIR)/tools

DISTNAME     := aegisrescue-$(VERSION)-$(ARCH)
ISO_OUT      := $(OUT_DIR)/$(DISTNAME).iso

# Rust 타겟
ifeq ($(ARCH),x86_64)
RUST_TARGET  := x86_64-unknown-linux-musl
else ifeq ($(ARCH),aarch64)
RUST_TARGET  := aarch64-unknown-linux-musl
else
RUST_TARGET  := $(ARCH)-unknown-linux-musl
endif

# 빌드 스크립트
BUILD_SH := $(BUILD_DIR)/build.sh

.PHONY: all iso rust gui squashfs chroot \
        test-qemu test-qemu-uefi test-qemu-forensic \
        write-usb docker-build docker-shell \
        check-deps preflight \
        clean distclean help

# ─── 기본 타겟 ────────────────────────────────────────────────
all: iso

# ─── 전체 ISO 빌드 ────────────────────────────────────────────
iso:
	@echo ""
	@echo "╔═══════════════════════════════════════════════════════╗"
	@echo "║        AegisRescue ISO 빌드 시작                      ║"
	@echo "║  버전: $(VERSION)  아키텍처: $(ARCH)  코어: $(JOBS)           ║"
	@echo "╚═══════════════════════════════════════════════════════╝"
	@echo ""
	@[ "$$(id -u)" = "0" ] || { echo "❌ root 권한 필요: sudo make iso"; exit 1; }
	@chmod +x $(BUILD_SH)
	@ARCH=$(ARCH) VERSION=$(VERSION) JOBS=$(JOBS) \
	 OUT_DIR=$(OUT_DIR) \
	 bash $(BUILD_SH)

# ─── 개별 타겟 ────────────────────────────────────────────────

# Rust 도구만 빌드 (root 불필요)
rust: $(OUT_DIR)
	@echo "=== Rust 도구 빌드 (target: $(RUST_TARGET)) ==="
	@cd $(PROJECT_ROOT) && \
	 RUSTFLAGS="-C target-feature=+crt-static" \
	 cargo build --release --jobs $(JOBS) \
	     --target $(RUST_TARGET) 2>&1 | \
	 grep -E "^(Compiling|Finished|error|warning:.*unused)" || true
	@mkdir -p $(TOOLS_DIR)
	@for bin in aegisd aegis aegis-scan aegis-mount aegis-recover \
	            aegis-boot aegis-forensic aegis-image aegis-ai \
	            aegis-sandbox aegis-preflight aegis-update; do \
	    src="$(PROJECT_ROOT)/target/$(RUST_TARGET)/release/$$bin"; \
	    [ -f "$$src" ] && { \
	        cp "$$src" $(TOOLS_DIR)/; \
	        strip $(TOOLS_DIR)/$$bin 2>/dev/null || true; \
	        echo "  ✓ $$bin ($$(du -sh $(TOOLS_DIR)/$$bin | cut -f1))"; \
	    } || echo "  ⚠ $$bin — 없음"; \
	done
	@echo "=== Rust 빌드 완료 ==="

# GUI 빌드 (AegisCenter + Installer)
gui: $(OUT_DIR)
	@echo "=== GUI 빌드 ==="
	@cd $(PROJECT_ROOT)/gui && npm install && cargo tauri build && \
	 cp src-tauri/target/release/aegiscenter $(TOOLS_DIR)/ 2>/dev/null || true
	@cd $(PROJECT_ROOT)/installer && npm install && cargo tauri build && \
	 cp src-tauri/target/release/aegis-installer $(TOOLS_DIR)/ 2>/dev/null || true
	@echo "=== GUI 빌드 완료 ==="

# squashfs만 재생성 (chroot가 이미 있을 때)
squashfs: $(OUT_DIR)
	@[ "$$(id -u)" = "0" ] || { echo "❌ root 권한 필요"; exit 1; }
	@[ -d "$(CHROOT_DIR)" ] || { echo "❌ chroot 없음 — 먼저 make iso 실행"; exit 1; }
	@echo "=== squashfs 생성 ==="
	@mkdir -p $(ISO_STAGE)/live
	@mksquashfs $(CHROOT_DIR) $(ISO_STAGE)/live/filesystem.squashfs \
	    -comp xz -Xbcj x86 -b 1M -processors $(JOBS) \
	    -ef $(BUILD_DIR)/squashfs-excludes.txt -noappend -progress
	@du -s --block-size=1 $(CHROOT_DIR) | cut -f1 > $(ISO_STAGE)/live/filesystem.size
	@echo "squashfs 크기: $$(du -sh $(ISO_STAGE)/live/filesystem.squashfs | cut -f1)"

# chroot만 재구성 (squashfs/ISO 없이)
chroot: $(OUT_DIR)
	@[ "$$(id -u)" = "0" ] || { echo "❌ root 권한 필요"; exit 1; }
	@chmod +x $(BUILD_SH)
	@ARCH=$(ARCH) VERSION=$(VERSION) JOBS=$(JOBS) \
	 OUT_DIR=$(OUT_DIR) \
	 bash $(BUILD_SH) --only-iso=0 --skip-kernel=0 2>&1 | head -20 || true

# ─── ISO만 재생성 (chroot + squashfs 기존 것 재사용) ─────────
iso-only: squashfs
	@echo "=== ISO 재생성 ==="
	@[ "$$(id -u)" = "0" ] || { echo "❌ root 권한 필요"; exit 1; }
	@chmod +x $(BUILD_SH)
	@ARCH=$(ARCH) VERSION=$(VERSION) JOBS=$(JOBS) \
	 OUT_DIR=$(OUT_DIR) \
	 bash $(BUILD_SH) --only-iso
	@echo "ISO: $(ISO_OUT)"

# ─── 테스트 ───────────────────────────────────────────────────

# BIOS 모드 QEMU 테스트
test-qemu: $(ISO_OUT)
	@command -v qemu-system-x86_64 >/dev/null || \
	    { echo "qemu 없음: apt install qemu-system-x86"; exit 1; }
	@echo "=== QEMU BIOS 테스트 ==="
	@echo "부팅 중: $(ISO_OUT)"
	@qemu-system-x86_64 \
	    -m 2G \
	    -smp 2 \
	    -cdrom $(ISO_OUT) \
	    -boot d \
	    -vga virtio \
	    -display sdl \
	    -net nic,model=virtio \
	    -net user \
	    -enable-kvm 2>/dev/null || \
	qemu-system-x86_64 \
	    -m 2G \
	    -smp 2 \
	    -cdrom $(ISO_OUT) \
	    -boot d \
	    -vga std

# UEFI 모드 QEMU 테스트
test-qemu-uefi: $(ISO_OUT)
	@command -v qemu-system-x86_64 >/dev/null || \
	    { echo "qemu 없음: apt install qemu-system-x86 ovmf"; exit 1; }
	@OVMF=$$(find /usr/share -name "OVMF.fd" 2>/dev/null | head -1); \
	 [ -n "$$OVMF" ] || OVMF=$$(find /usr/share -name "OVMF_CODE.fd" 2>/dev/null | head -1); \
	 [ -n "$$OVMF" ] || { echo "OVMF 없음: apt install ovmf"; exit 1; }; \
	 echo "=== QEMU UEFI 테스트 ($$OVMF) ==="; \
	 qemu-system-x86_64 \
	     -m 2G \
	     -smp 2 \
	     -bios "$$OVMF" \
	     -cdrom $(ISO_OUT) \
	     -boot d \
	     -vga virtio \
	     -display sdl \
	     -enable-kvm 2>/dev/null || \
	 qemu-system-x86_64 \
	     -m 2G \
	     -bios "$$OVMF" \
	     -cdrom $(ISO_OUT) \
	     -vga std

# 포렌식 모드 QEMU 테스트
test-qemu-forensic: $(ISO_OUT)
	@echo "=== QEMU 포렌식 모드 테스트 ==="
	@qemu-system-x86_64 \
	    -m 2G \
	    -cdrom $(ISO_OUT) \
	    -boot d \
	    -append "boot=live aegis.mode=forensic noswap" \
	    -vga std 2>/dev/null || true

# ─── USB 기록 ─────────────────────────────────────────────────
write-usb: $(ISO_OUT)
	@[ -n "$(USB)" ] || { \
	    echo "사용법: make write-usb USB=/dev/sdX"; \
	    echo "주의: 선택한 장치의 모든 데이터가 삭제됩니다!"; \
	    lsblk -o NAME,SIZE,TYPE,VENDOR,MODEL; \
	    exit 1; }
	@[ "$$(id -u)" = "0" ] || { echo "root 권한 필요"; exit 1; }
	@echo "경고: $(USB)의 모든 데이터가 삭제됩니다."
	@read -p "계속하려면 'yes' 입력: " confirm && [ "$$confirm" = "yes" ]
	@echo "기록 중: $(ISO_OUT) → $(USB)"
	@dd if=$(ISO_OUT) of=$(USB) bs=4M status=progress oflag=sync
	@sync
	@echo "✓ USB 준비 완료: $(USB)"

# ─── Docker 빌드 ──────────────────────────────────────────────
docker-build:
	@echo "=== Docker 빌드 환경 구성 ==="
	@docker build -t aegisrescue-builder -f $(BUILD_DIR)/Dockerfile $(BUILD_DIR)/
	@docker run --rm --privileged \
	    -v $(PROJECT_ROOT):/workspace \
	    -e ARCH=$(ARCH) \
	    -e VERSION=$(VERSION) \
	    -e JOBS=$(JOBS) \
	    aegisrescue-builder
	@echo "=== Docker 빌드 완료 ==="

docker-shell:
	@docker run --rm --privileged -it \
	    -v $(PROJECT_ROOT):/workspace \
	    aegisrescue-builder bash

# ─── 유틸리티 ─────────────────────────────────────────────────
check-deps:
	@echo "=== 의존성 확인 ==="
	@for dep in debootstrap xorriso mksquashfs grub-mkrescue \
	            xz lz4 zstd rsync sha256sum cargo node npm; do \
	    if command -v $$dep >/dev/null 2>&1; then \
	        echo "  ✓ $$dep ($$($$dep --version 2>&1 | head -1))"; \
	    else \
	        echo "  ✗ $$dep — 없음"; \
	    fi; \
	done

preflight:
	@[ -f "$(TOOLS_DIR)/aegis-preflight" ] && \
	    $(TOOLS_DIR)/aegis-preflight || \
	    { cargo run -p aegis-preflight --release 2>/dev/null || \
	      echo "aegis-preflight 없음 — make rust 먼저 실행"; }

$(OUT_DIR):
	@mkdir -p $(OUT_DIR)

# ─── 정리 ─────────────────────────────────────────────────────
clean:
	@echo "=== 빌드 아티팩트 정리 ==="
	@cd $(PROJECT_ROOT) && cargo clean 2>/dev/null || true
	@rm -rf $(TOOLS_DIR) $(ISO_STAGE) $(OUT_DIR)/$(DISTNAME).*
	@echo "정리 완료"

distclean: clean
	@echo "=== chroot 포함 전체 삭제 ==="
	@[ "$$(id -u)" = "0" ] || { echo "root 권한 필요 (chroot 마운트 해제)"; exit 1; }
	@umount -l $(CHROOT_DIR)/dev/pts $(CHROOT_DIR)/dev \
	            $(CHROOT_DIR)/sys $(CHROOT_DIR)/proc 2>/dev/null || true
	@rm -rf $(CHROOT_DIR) $(ISO_STAGE)
	@echo "distclean 완료"

# ─── 도움말 ───────────────────────────────────────────────────
help:
	@echo ""
	@echo "AegisRescue 빌드 시스템"
	@echo ""
	@echo "기본 타겟:"
	@echo "  make              전체 ISO 빌드 (sudo 필요)"
	@echo "  make iso          전체 ISO 빌드"
	@echo "  make rust         Rust 도구만 빌드"
	@echo "  make gui          GUI만 빌드"
	@echo "  make squashfs     squashfs만 재생성"
	@echo "  make iso-only     ISO만 재생성 (chroot 재사용)"
	@echo ""
	@echo "테스트:"
	@echo "  make test-qemu         BIOS 모드 QEMU 테스트"
	@echo "  make test-qemu-uefi    UEFI 모드 QEMU 테스트"
	@echo ""
	@echo "배포:"
	@echo "  make write-usb USB=/dev/sdX    USB에 기록"
	@echo "  make docker-build              Docker 컨테이너로 빌드"
	@echo ""
	@echo "환경 변수:"
	@echo "  ARCH=x86_64|aarch64    대상 아키텍처 (현재: $(ARCH))"
	@echo "  VERSION=0.1.0          버전 (현재: $(VERSION))"
	@echo "  JOBS=N                 병렬 빌드 수 (현재: $(JOBS))"
	@echo ""
	@echo "예시:"
	@echo "  sudo make                         x86_64 전체 빌드"
	@echo "  sudo make iso ARCH=aarch64        ARM64 빌드"
	@echo "  make rust                         Rust 도구만 빌드"
	@echo "  make test-qemu                    QEMU 테스트"
	@echo "  sudo make write-usb USB=/dev/sdb  USB 플래시 기록"
