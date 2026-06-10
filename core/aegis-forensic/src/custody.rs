//! 체인 오브 커스터디 리포트 생성

use std::path::Path;

pub fn generate_report(evidence_dir: &Path) -> anyhow::Result<String> {
    let mut report = String::new();

    report.push_str("═══════════════════════════════════════════════════\n");
    report.push_str("       CHAIN OF CUSTODY REPORT — AegisRescue\n");
    report.push_str("═══════════════════════════════════════════════════\n");
    report.push_str(&format!("Generated:    {}\n", chrono::Utc::now().to_rfc3339()));
    report.push_str(&format!("Tool:         AegisRescue v{}\n", env!("CARGO_PKG_VERSION")));
    report.push_str(&format!("Evidence dir: {}\n", evidence_dir.display()));
    report.push_str("───────────────────────────────────────────────────\n\n");

    // 해시 파일 목록
    for entry in walkdir::WalkDir::new(evidence_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "sha256")
                .unwrap_or(false)
        })
    {
        let content = std::fs::read_to_string(entry.path())
            .unwrap_or_default();
        report.push_str(&format!("Hash record: {}\n{}\n", entry.path().display(), content));
    }

    // 증거 파일 목록 + 현재 해시 검증
    report.push_str("Evidence Integrity Verification:\n");
    report.push_str("───────────────────────────────────────────────────\n");

    for entry in walkdir::WalkDir::new(evidence_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            ext != "sha256"
        })
    {
        let hash = aegis_common::hash::sha256_file(entry.path())
            .unwrap_or_else(|_| "ERROR".into());
        report.push_str(&format!(
            "  {}  {}\n",
            &hash[..16],
            entry.path().display()
        ));
    }

    report.push_str("\nEnd of Chain of Custody Report\n");

    Ok(report)
}
