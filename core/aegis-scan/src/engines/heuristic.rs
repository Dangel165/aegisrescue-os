//! 정적 휴리스틱 엔진 — PE/ELF 분석 + 엔트로피 + 패킹 탐지

use aegis_common::hash::bytes_entropy;
use aegis_common::types::{DetectionEngine, DetectionResult};
use goblin::Object;
use std::path::Path;

const HIGH_ENTROPY_THRESHOLD: f64 = 7.2;
const PACKER_THRESHOLD: f64 = 7.6;

pub struct HeuristicEngine;

impl HeuristicEngine {
    pub fn new() -> Self { Self }

    pub fn scan(&self, path: &Path, data: &[u8]) -> Option<DetectionResult> {
        let mut score = 0.0f64;
        let mut indicators = vec![];

        // 엔트로피 분석
        let entropy = bytes_entropy(data);
        if entropy > PACKER_THRESHOLD {
            score += 0.4;
            indicators.push(format!("High entropy ({:.2}) — packed/encrypted binary", entropy));
        } else if entropy > HIGH_ENTROPY_THRESHOLD {
            score += 0.2;
            indicators.push(format!("Elevated entropy ({:.2})", entropy));
        }

        // PE 파일 분석
        if let Ok(Object::PE(pe)) = Object::parse(data) {
            score += analyze_pe(&pe, &mut indicators);
        }

        // ELF 분석
        if let Ok(Object::Elf(elf)) = Object::parse(data) {
            score += analyze_elf(&elf, &mut indicators);
        }

        // 의심 문자열 패턴
        score += scan_strings(data, &mut indicators);

        if score < 0.3 { return None; }

        Some(DetectionResult {
            engine: DetectionEngine::Heuristic,
            signature: Some(format!("Heuristic.Generic:{}", path.file_name().unwrap_or_default().to_string_lossy())),
            confidence: score.min(1.0),
            details: Some(format!("{} — {}", path.display(), indicators.join("; "))),
        })
    }
}

impl super::Engine for HeuristicEngine {
    fn name(&self) -> &'static str {
        "heuristic"
    }

    fn scan_bytes(&self, data: &[u8]) -> super::EngineResult {
        self.scan(Path::new("<memory>"), data)
    }
}

fn analyze_pe(pe: &goblin::pe::PE, indicators: &mut Vec<String>) -> f64 {
    let mut score = 0.0f64;

    // 자기 서명 없음 (일반적으로 합법 소프트웨어는 서명)
    // 섹션 수 이상
    if pe.sections.len() < 2 {
        score += 0.1;
        indicators.push("Very few PE sections".into());
    }

    // .text 섹션 실행 가능 여부 + 쓰기 가능 (W^X 위반)
    for section in &pe.sections {
        let chars = section.characteristics;
        let writable = chars & 0x80000000 != 0;
        let executable = chars & 0x20000000 != 0;
        if writable && executable {
            score += 0.3;
            let name = std::str::from_utf8(&section.name).unwrap_or("?");
            indicators.push(format!("Section '{}' is both W+X", name.trim_end_matches('\0')));
        }
    }

    // 의심 임포트
    let suspicious_imports = [
        "VirtualAlloc", "WriteProcessMemory", "CreateRemoteThread",
        "ShellExecute", "WinExec", "LoadLibrary",
        "RegSetValue", "RegCreateKey",
        "IsDebuggerPresent", "CheckRemoteDebuggerPresent",
    ];

    let imports_lower: Vec<String> = pe.imports.iter()
        .map(|i| i.name.to_lowercase())
        .collect();

    let mut suspicious_count = 0;
    for &suspect in &suspicious_imports {
        if imports_lower.contains(&suspect.to_lowercase()) {
            suspicious_count += 1;
        }
    }

    if suspicious_count >= 3 {
        score += 0.2;
        indicators.push(format!("{} suspicious imports", suspicious_count));
    }

    score
}

fn analyze_elf(elf: &goblin::elf::Elf, indicators: &mut Vec<String>) -> f64 {
    let mut score = 0.0f64;

    // 동적 링킹 없이 독립 실행 + 작은 크기 (드로퍼 패턴)
    if elf.dynamic.is_none() && elf.entry != 0 {
        score += 0.1;
        indicators.push("Statically linked ELF (potential dropper)".into());
    }

    score
}

fn scan_strings(data: &[u8], indicators: &mut Vec<String>) -> f64 {
    let text = String::from_utf8_lossy(data);
    let text_lower = text.to_lowercase();
    let mut score = 0.0f64;

    let suspicious_strings = [
        ("ransom", 0.5, "Ransomware keyword"),
        ("your files have been encrypted", 0.8, "Ransomware message"),
        ("bitcoin", 0.2, "Cryptocurrency reference"),
        ("cmd.exe /c", 0.2, "CMD execution"),
        ("powershell -enc", 0.3, "Encoded PowerShell"),
        ("invoke-expression", 0.3, "PowerShell IEX"),
        ("wscript.shell", 0.2, "WScript shell"),
        ("http://", 0.1, "HTTP URL"),
        ("\\\\pipe\\\\", 0.2, "Named pipe (C2 pattern)"),
        ("delete shadow", 0.6, "Shadow copy deletion (ransomware)"),
        ("vssadmin", 0.4, "VSS administration"),
        ("bcdedit /set", 0.4, "Boot config modification"),
        ("disable windows defender", 0.5, "AV evasion"),
    ];

    for (pattern, weight, desc) in &suspicious_strings {
        if text_lower.contains(pattern) {
            score += weight;
            indicators.push(desc.to_string());
        }
    }

    score.min(0.6) // 문자열만으로 최대 0.6
}
