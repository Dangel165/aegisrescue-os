//! 공개 AI 분류 API

pub mod features;
pub mod inference;
pub mod model;
pub mod result;

#[cfg(feature = "onnx")]
pub mod onnx_tract;

pub use model::AiClassifier;
pub use result::AiResult;
