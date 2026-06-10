//! 정적 특징 추출기
//!
//! PE/ELF 파일에서 AI 모델 입력 특징 벡터 추출

use goblin::Object;
use std::path::Path;

/// AI 모델 입력 특징 벡터
#[derive(Debug, Clone)]
pub struct FeatureVector {
    /// 파일 크기 (정규화)
    pub size_normalized: f32,
    /// 전체 엔트로피 (0~8 → 0~1 정규화)
    pub entropy: f32,
    /// 바이트 히스토그램 (256개 빈, 정규화)
    pub byte_histogram: Vec<f32>,
    /// 섹션 수
    pub section_count: f32,
    /// 임포트 API 수
    pub import_count: f32,
    /// W+X 섹션 여부
    pub has_wx_section: f32,
    /// 패킹 지표
    pub packing_score: f32,
    /// 의심 문자열 수
    pub suspicious_strings: f32,
    /// 디버그 정보 제거됨
    pub stripped: f32,
    /// 디지털 서명 없음
    pub unsigned: f32,
    /// 네트워크 관련 임포트
    pub network_imports: f32,
    /// 프로세스 관련 임포트
    pub process_imports: f32,
    /// 파일 시스템 관련 임포트
    pub file_imports: f32,
    /// 암호화 관련 임포트
    pub crypto_imports: f32,
    /// 레지스트리 관련 임포트
    pub registry_imports: f32,
}

impl FeatureVector {
    pub fn extract(path: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self::from_bytes(&data))
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        let entropy = aegis_common::hash::bytes_entropy(data) as f32 / 8.0;
        let byte_histogram = compute_histogram(data);
        let size_normalized = (data.len() as f32 / (100.0 * 1024.0 * 1024.0)).min(1.0);
        let suspicious_strings = count_suspicious_strings(data) as f32 / 20.0;

        let mut fv = FeatureVector {
            size_normalized,
            entropy,
            byte_histogram,
            section_count: 0.0,
            import_count: 0.0,
            has_wx_section: 0.0,
            packing_score: if entropy > 0.9 { 1.0 } else { 0.0 },
            suspicious_strings,
            stripped: 0.0,
            unsigned: 1.0, // 기본: 서명 없음
            network_imports: 0.0,
            process_imports: 0.0,
            file_imports: 0.0,
            crypto_imports: 0.0,
            registry_imports: 0.0,
        };

        // 바이너리 포맷별 추가 특징 추출
        if let Ok(obj) = Object::parse(data) {
            match obj {
                Object::PE(pe) => extract_pe_features(&pe, &mut fv),
                Object::Elf(elf) => extract_elf_features(&elf, &mut fv),
                _ => {}
            }
        }

        fv
    }

    /// f32 배열로 변환 (모델 입력용)
    pub fn to_vec(&self) -> Vec<f32> {
        let mut v = vec![
            self.size_normalized,
            self.entropy,
            self.section_count,
            self.import_count,
            self.has_wx_section,
            self.packing_score,
            self.suspicious_strings,
            self.stripped,
            self.unsigned,
            self.network_imports,
            self.process_imports,
            self.file_imports,
            self.crypto_imports,
            self.registry_imports,
        ];
        v.extend_from_slice(&self.byte_histogram);
        v
    }

    pub const FEATURE_DIM: usize = 14 + 256;
}

fn compute_histogram(data: &[u8]) -> Vec<f32> {
    let mut hist = vec![0u64; 256];
    for &b in data {
        hist[b as usize] += 1;
    }
    let len = data.len() as f32;
    hist.iter().map(|&c| c as f32 / len).collect()
}

fn count_suspicious_strings(data: &[u8]) -> usize {
    let text = String::from_utf8_lossy(data);
    let text_lower = text.to_lowercase();
    let patterns = [
        "ransom", "encrypt", "decrypt", "bitcoin", "wallet",
        "cmd.exe", "powershell", "vssadmin", "shadow",
        "http://", "https://", "base64",
    ];
    patterns.iter().filter(|p| text_lower.contains(**p)).count()
}

fn extract_pe_features(pe: &goblin::pe::PE, fv: &mut FeatureVector) {
    fv.section_count = pe.sections.len() as f32 / 10.0;
    fv.import_count = pe.imports.len() as f32 / 100.0;

    for s in &pe.sections {
        let c = s.characteristics;
        if c & 0x80000000 != 0 && c & 0x20000000 != 0 {
            fv.has_wx_section = 1.0;
        }
    }

    // 임포트 카테고리 분류
    let imports_lower: Vec<String> = pe.imports.iter()
        .map(|i| i.name.to_lowercase())
        .collect();

    let network_api = ["winsock", "wsasend", "connect", "httpopen", "internetopen", "urldownload"];
    let process_api = ["createprocess", "virtualalloc", "writeprocessmemory", "createremotethread", "ntcreatethread"];
    let file_api    = ["createfile", "writefile", "deletefile", "movefile", "copyfile"];
    let crypto_api  = ["cryptencrypt", "cryptdecrypt", "bcryptencrypt", "cngencerate", "aes"];
    let reg_api     = ["regcreate", "regsetvalue", "regopen", "regenumkey"];

    fv.network_imports  = count_matching(&imports_lower, &network_api) as f32 / 5.0;
    fv.process_imports  = count_matching(&imports_lower, &process_api) as f32 / 5.0;
    fv.file_imports     = count_matching(&imports_lower, &file_api) as f32 / 5.0;
    fv.crypto_imports   = count_matching(&imports_lower, &crypto_api) as f32 / 5.0;
    fv.registry_imports = count_matching(&imports_lower, &reg_api) as f32 / 5.0;
}

fn extract_elf_features(elf: &goblin::elf::Elf, fv: &mut FeatureVector) {
    fv.import_count = elf.dynsyms.iter().count() as f32 / 100.0;
    fv.stripped = if elf.syms.is_empty() { 1.0 } else { 0.0 };
}

fn count_matching(list: &[String], patterns: &[&str]) -> usize {
    patterns.iter()
        .filter(|p| list.iter().any(|s| s.contains(**p)))
        .count()
}
