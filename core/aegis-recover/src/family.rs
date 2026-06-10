//! 랜섬웨어 패밀리 식별

use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct RansomwareFamily {
    pub name: String,
    pub extension: Option<String>,
    pub ransom_note_patterns: Vec<String>,
    pub has_decryptor: bool,
    pub decryptor_url: Option<String>,
    pub notes: String,
}

/// 알려진 랜섬웨어 패밀리 데이터베이스
fn known_families() -> Vec<RansomwareFamily> {
    vec![
        RansomwareFamily {
            name: "WannaCry".into(),
            extension: Some("WNCRY".into()),
            ransom_note_patterns: vec!["@WanaDecryptor@".into(), "!Please Read Me!".into()],
            has_decryptor: true,
            decryptor_url: Some("https://www.nomoreransom.org/en/decryption-tools.html".into()),
            notes: "Exploits EternalBlue. WanaKiwi/WannakeyGUI may work on unpatched memory.".into(),
        },
        RansomwareFamily {
            name: "GandCrab".into(),
            extension: Some("GDCB".into()),
            ransom_note_patterns: vec!["GDCB-DECRYPT.txt".into()],
            has_decryptor: true,
            decryptor_url: Some("https://www.nomoreransom.org".into()),
            notes: "Multiple versions, decryptor available from Bitdefender/Europol.".into(),
        },
        RansomwareFamily {
            name: "Ryuk".into(),
            extension: Some("RYK".into()),
            ransom_note_patterns: vec!["RyukReadMe.txt".into()],
            has_decryptor: false,
            decryptor_url: None,
            notes: "Targets enterprises. No public decryptor. Focus on backups.".into(),
        },
        RansomwareFamily {
            name: "REvil/Sodinokibi".into(),
            extension: None,
            ransom_note_patterns: vec!["-readme.txt".into(), "---README---.txt".into()],
            has_decryptor: true,
            decryptor_url: Some("https://www.nomoreransom.org".into()),
            notes: "Master decryptor released by Bitdefender after law enforcement action.".into(),
        },
        RansomwareFamily {
            name: "Conti".into(),
            extension: Some("CONTI".into()),
            ransom_note_patterns: vec!["CONTI_README.txt".into(), "readme.txt".into()],
            has_decryptor: true,
            decryptor_url: Some("https://github.com/QuietSwind/Conti-Decryptor".into()),
            notes: "Leaked source and keys. Partial decryptor available.".into(),
        },
        RansomwareFamily {
            name: "LockBit".into(),
            extension: Some("lockbit".into()),
            ransom_note_patterns: vec!["Restore-My-Files.txt".into(), "!!!-Restore-My-Files-!!!.txt".into()],
            has_decryptor: false,
            decryptor_url: None,
            notes: "Active RaaS. No public decryptor yet.".into(),
        },
        RansomwareFamily {
            name: "STOP/Djvu".into(),
            extension: None, // .djvu, .stop, .rezm, etc.
            ransom_note_patterns: vec!["_readme.txt".into()],
            has_decryptor: true,
            decryptor_url: Some("https://www.emsisoft.com/ransomware-decryption-tools/stop-djvu".into()),
            notes: "Most prolific ransomware. Emsisoft offers free decryptor for offline keys.".into(),
        },
        RansomwareFamily {
            name: "Dharma/CrySiS".into(),
            extension: None, // .[email].id
            ransom_note_patterns: vec!["FILES ENCRYPTED.txt".into()],
            has_decryptor: true,
            decryptor_url: Some("https://www.kaspersky.com/ransomware-decryptor".into()),
            notes: "Some variants decryptable via Kaspersky/Avast tools.".into(),
        },
    ]
}

pub fn identify_ransomware(target: &Path) -> Option<RansomwareFamily> {
    let families = known_families();

    // 1) 랜섬노트 파일명 매칭
    for entry in WalkDir::new(target).max_depth(3)
        .into_iter().filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        for fam in &families {
            for pattern in &fam.ransom_note_patterns {
                if name.contains(&pattern.to_lowercase()) {
                    tracing::info!(family = fam.name, "ransomware identified by note");
                    return Some(fam.clone());
                }
            }
        }
    }

    // 2) 파일 확장자 매칭
    let encrypted_ext = find_most_common_encrypted_ext(target);
    if let Some(ref ext) = encrypted_ext {
        for fam in &families {
            if let Some(ref fe) = fam.extension {
                if ext.to_lowercase().contains(&fe.to_lowercase()) {
                    tracing::info!(family = fam.name, ext = ext, "ransomware identified by extension");
                    return Some(fam.clone());
                }
            }
        }

        // STOP/Djvu 패턴: 4자리 알파벳 확장자
        if ext.len() == 4 && ext.chars().all(|c| c.is_alphabetic()) {
            return families.iter().find(|f| f.name.starts_with("STOP")).cloned();
        }
    }

    None
}

/// 가장 많이 나타나는 의심 확장자 탐색
fn find_most_common_encrypted_ext(target: &Path) -> Option<String> {
    use std::collections::HashMap;
    let mut ext_counts: HashMap<String, usize> = HashMap::new();

    for entry in WalkDir::new(target).max_depth(4)
        .into_iter().filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            let ext = ext.to_lowercase();
            // 알려진 정상 확장자 제외
            if !is_common_extension(&ext) {
                *ext_counts.entry(ext).or_default() += 1;
            }
        }
    }

    // 가장 많이 나타나는 낯선 확장자
    ext_counts.into_iter()
        .filter(|(_, c)| *c > 5)
        .max_by_key(|(_, c)| *c)
        .map(|(ext, _)| ext)
}

fn is_common_extension(ext: &str) -> bool {
    matches!(ext,
        "txt" | "doc" | "docx" | "xls" | "xlsx" | "pdf" | "jpg" | "jpeg" |
        "png" | "gif" | "mp4" | "mp3" | "zip" | "rar" | "exe" | "dll" |
        "sys" | "ini" | "cfg" | "log" | "bat" | "cmd" | "ps1" | "py"
    )
}
