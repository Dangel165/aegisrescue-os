//! 손상 디스크 복구 복사 (ddrescue 래퍼)

use std::path::Path;

pub struct DdrescueStats {
    pub rescued_sectors: u64,
    pub error_sectors: u64,
    pub size_bytes: u64,
}

pub async fn image_damaged_disk(
    source: &Path,
    dest: &Path,
) -> anyhow::Result<DdrescueStats> {
    let log_file = dest.with_extension("ddrescue.log");

    tracing::info!(
        source = %source.display(),
        dest = %dest.display(),
        "starting ddrescue imaging"
    );

    // 1차 패스: 오류 없는 영역 우선 복사
    let source_arg = source.to_string_lossy().into_owned();
    let dest_arg = dest.to_string_lossy().into_owned();
    let log_arg = log_file.to_string_lossy().into_owned();
    let status = tokio::process::Command::new("ddrescue")
        .args([
            "--force",
            "--no-split",           // 1차: 빠른 복사
            "--direct",             // 캐시 우회
            source_arg.as_str(),
            dest_arg.as_str(),
            log_arg.as_str(),
        ])
        .status()
        .await;

    if let Err(e) = &status {
        tracing::warn!("ddrescue not available: {}. Falling back to dd.", e);
        return fallback_dd(source, dest).await;
    }

    // 2차 패스: 오류 섹터 재시도 (3회)
    for pass in 2..=4 {
        tracing::info!(pass = pass, "ddrescue retry pass");
        let retry_pass = format!("--retry-passes={}", pass);
        let _ = tokio::process::Command::new("ddrescue")
            .args([
                "--force",
                "--direct",
                retry_pass.as_str(),
                source_arg.as_str(),
                dest_arg.as_str(),
                log_arg.as_str(),
            ])
            .status()
            .await;
    }

    // 로그 파싱
    let stats = parse_ddrescue_log(&log_file).unwrap_or(DdrescueStats {
        rescued_sectors: 0,
        error_sectors: 0,
        size_bytes: std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0),
    });

    Ok(stats)
}

async fn fallback_dd(source: &Path, dest: &Path) -> anyhow::Result<DdrescueStats> {
    let size = std::fs::metadata(source)
        .map(|m| m.len())
        .unwrap_or(0);

    let input = format!("if={}", source.display());
    let output = format!("of={}", dest.display());
    let status = tokio::process::Command::new("dd")
        .args([
            input.as_str(),
            output.as_str(),
            "bs=4096",
            "conv=noerror,sync",
            "status=progress",
        ])
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::anyhow!("dd fallback failed"));
    }

    Ok(DdrescueStats {
        rescued_sectors: size / 512,
        error_sectors: 0,
        size_bytes: size,
    })
}

fn parse_ddrescue_log(log_file: &Path) -> Option<DdrescueStats> {
    let content = std::fs::read_to_string(log_file).ok()?;
    let mut rescued = 0u64;
    let mut errors = 0u64;
    let mut total_size = 0u64;

    for line in content.lines() {
        // ddrescue 로그 형식 파싱
        // "# Mapfile. Created by GNU ddrescue version X.X"
        // "0x0    0x...  +"  또는  "-"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && !line.starts_with('#') {
            if let (Ok(start), Ok(size)) = (
                u64::from_str_radix(parts[0].trim_start_matches("0x"), 16),
                u64::from_str_radix(parts[1].trim_start_matches("0x"), 16),
            ) {
                total_size = total_size.max(start + size);
                match parts[2] {
                    "+" => rescued += size / 512,
                    "-" | "/" | "*" => errors += size / 512,
                    _ => {}
                }
            }
        }
    }

    Some(DdrescueStats {
        rescued_sectors: rescued,
        error_sectors: errors,
        size_bytes: total_size,
    })
}
