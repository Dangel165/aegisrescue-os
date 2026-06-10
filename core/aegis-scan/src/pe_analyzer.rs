//! PE/ELF 정적 분석 유틸리티

use goblin::Object;
use std::path::Path;

#[derive(Debug)]
pub struct BinaryInfo {
    pub format: BinaryFormat,
    pub architecture: String,
    pub is_packed: bool,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub strings: Vec<String>,
    pub entropy: f64,
}

#[derive(Debug, PartialEq)]
pub enum BinaryFormat {
    Pe,
    Elf,
    MachO,
    Unknown,
}

pub fn analyze(path: &Path) -> Option<BinaryInfo> {
    let data = std::fs::read(path).ok()?;
    let entropy = aegis_common::hash::bytes_entropy(&data);

    match Object::parse(&data).ok()? {
        Object::PE(pe) => Some(analyze_pe(&pe, &data, entropy)),
        Object::Elf(elf) => Some(analyze_elf(&elf, &data, entropy)),
        _ => None,
    }
}

fn analyze_pe(pe: &goblin::pe::PE, data: &[u8], entropy: f64) -> BinaryInfo {
    let imports: Vec<String> = pe.imports.iter()
        .map(|i| format!("{}!{}", i.dll, i.name))
        .collect();

    let exports: Vec<String> = pe.exports.iter()
        .filter_map(|e| e.name.map(|n| n.to_string()))
        .collect();

    let arch = if pe.is_64 { "x86_64" } else { "x86" }.to_string();

    // 패킹 감지: 섹션 엔트로피 또는 섹션 이름 이상
    let is_packed = entropy > 7.2 || pe.sections.iter().any(|s| {
        let name = std::str::from_utf8(&s.name).unwrap_or("").trim_end_matches('\0');
        matches!(name, ".upx0" | ".upx1" | ".aspack" | ".nsp0" | ".mpress1")
    });

    let strings = extract_strings(data);

    BinaryInfo {
        format: BinaryFormat::Pe,
        architecture: arch,
        is_packed,
        imports,
        exports,
        strings,
        entropy,
    }
}

fn analyze_elf(elf: &goblin::elf::Elf, data: &[u8], entropy: f64) -> BinaryInfo {
    let imports: Vec<String> = elf.dynsyms.iter()
        .filter_map(|s| elf.dynstrtab.get_at(s.st_name).map(|n| n.to_string()))
        .collect();

    let arch = match elf.header.e_machine {
        goblin::elf::header::EM_X86_64  => "x86_64",
        goblin::elf::header::EM_386     => "x86",
        goblin::elf::header::EM_AARCH64 => "aarch64",
        goblin::elf::header::EM_ARM     => "arm",
        _ => "unknown",
    }.to_string();

    BinaryInfo {
        format: BinaryFormat::Elf,
        architecture: arch,
        is_packed: entropy > 7.4,
        imports,
        exports: vec![],
        strings: extract_strings(data),
        entropy,
    }
}

/// 바이너리에서 출력 가능한 문자열 추출 (최소 길이 4)
fn extract_strings(data: &[u8]) -> Vec<String> {
    let mut strings = vec![];
    let mut current = String::new();

    for &b in data {
        if b.is_ascii_graphic() || b == b' ' {
            current.push(b as char);
        } else {
            if current.len() >= 4 {
                strings.push(current.clone());
            }
            current.clear();
        }
    }

    // 상위 500개만 (메모리 제한)
    strings.truncate(500);
    strings
}
