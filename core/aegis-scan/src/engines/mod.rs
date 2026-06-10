pub mod heuristic;
pub mod ioc;
pub mod signature;
pub mod yara;

use aegis_common::types::DetectionResult;

pub trait Engine: Send + Sync {
    fn name(&self) -> &'static str;
    fn scan_bytes(&self, data: &[u8]) -> EngineResult;
}

pub type EngineResult = Option<DetectionResult>;
