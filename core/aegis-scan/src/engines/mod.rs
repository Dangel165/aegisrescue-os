pub mod heuristic;
pub mod ioc;
pub mod signature;
pub mod yara;

use aegis_common::types::DetectionResult;
use std::path::Path;

pub trait Engine: Send + Sync {
    fn name(&self) -> &'static str;
    fn scan_bytes(&self, data: &[u8]) -> Option<DetectionResult>;
}

pub type EngineResult = Option<DetectionResult>;
