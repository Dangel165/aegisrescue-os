//! AegisRescue 공용 라이브러리
//!
//! 모든 aegis-* 크레이트가 공유하는 타입, 유틸리티, IPC 프로토콜,
//! 에러 정의, 해시 유틸리티, 로깅 초기화 등을 제공한다.

pub mod error;
pub mod hash;
pub mod ipc;
pub mod policy;
pub mod report;
pub mod types;
pub mod utils;

pub use error::{AegisError, Result};
pub use types::*;
