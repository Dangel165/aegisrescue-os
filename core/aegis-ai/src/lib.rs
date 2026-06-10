//! AegisRescue AI 악성코드 분류 라이브러리
//!
//! 완전 오프라인 추론 — 2GB RAM 환경에서 동작 가능한 경량 모델 우선

pub mod features;
pub mod model;
pub mod result;

pub use model::AiClassifier;
pub use result::AiResult;
