use chrono::Utc;
use std::path::{Path, PathBuf};

pub fn backup_boot_record(device: &Path, output_dir: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;

    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let device_name = device.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let filename = format!("{}_bootrec_{}.bin", device_name, ts);
    let dest = output_dir.join(&filename);

    // 첫 2048 섹터 (1MiB) — MBR + GPT 헤더 + 파티션 엔트리
    let status = std::process::Command::new("dd")
        .args([
            &format!("if={}", device.display()),
            &format!("of={}", dest.display()),
            "bs=512",
            "count=2048",
            "status=none",
        ])
        .status()
        .map_err(|_| anyhow::anyhow!("dd not available"))?;

    if !status.success() {
        // dd 없는 환경: 직접 읽기
        let mut f = std::fs::File::open(device)?;
        let mut buf = vec![0u8; 512 * 2048];
        use std::io::Read;
        let n = f.read(&mut buf)?;
        std::fs::write(&dest, &buf[..n])?;
    }

    // 해시 기록
    let hash = aegis_common::hash::sha256_file(&dest)?;
    let hash_file = dest.with_extension("sha256");
    std::fs::write(&hash_file, format!("{}  {}\n", hash, filename))?;

    tracing::info!(
        backup = %dest.display(),
        hash = %&hash[..16],
        "boot record backed up"
    );

    Ok(dest)
}
