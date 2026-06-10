# AegisRescue 개발 참여 가이드

## 개발 환경 설정

```bash
# 1. Rust 툴체인
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

# 2. 저장소 클론
git clone https://github.com/aegisrescue/aegisrescue
cd aegisrescue

# 3. 의존성 확인
cargo check --workspace

# 4. 테스트 실행
cargo test --workspace
```

## 코드 스타일

- **Rust 우선**: 신규 기능은 모두 Rust로 작성
- `cargo fmt` 및 `cargo clippy` 통과 필수
- `unsafe` 코드는 최소화, 반드시 주석으로 근거 설명
- 에러는 `thiserror` 사용, `unwrap()`/`expect()` 금지 (테스트 제외)

## 보안 규칙

- 감염 디스크 데이터를 파싱하는 코드는 퍼징 테스트 필수
- 새 마운트 경로는 정책 검사 후에만 허용
- 사용자 입력은 경로 순회 공격 방어 필수

## PR 체크리스트

- [ ] 새 기능에 대한 테스트 추가
- [ ] `cargo test` 통과
- [ ] `cargo clippy -- -D warnings` 통과
- [ ] `cargo fmt --check` 통과
- [ ] 보안 관련 변경은 별도 리뷰 요청
