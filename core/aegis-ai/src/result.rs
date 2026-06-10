use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResult {
    pub class: MalwareClass,
    /// 0.0 ~ 1.0
    pub confidence: f32,
    /// 클래스별 확률 [(class_name, probability)]
    pub probabilities: Vec<(String, f32)>,
    /// 판정 근거 특징 목록 (설명가능성)
    pub key_features: Vec<String>,
    pub model_version: String,
}

impl AiResult {
    pub fn is_malicious(&self) -> bool {
        self.class != MalwareClass::Clean && self.confidence > 0.5
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MalwareClass {
    Ransomware,
    Rootkit,
    Trojan,
    Worm,
    Backdoor,
    Cryptominer,
    PotentiallyUnwanted,
    Clean,
}

impl std::fmt::Display for MalwareClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
