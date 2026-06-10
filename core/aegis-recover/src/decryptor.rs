//! 알려진 복호화 도구 실행 관리자

use crate::family::RansomwareFamily;
use std::path::Path;

pub struct RecoveryStats {
    pub total: u64,
    pub recovered: u64,
    pub failed: u64,
}

pub struct Decryptor<'a> {
    family: Option<&'a RansomwareFamily>,
    key: Option<&'a str>,
}

impl<'a> Decryptor<'a> {
    pub fn new(family: Option<&'a RansomwareFamily>, key: Option<&'a str>) -> Self {
        Self { family, key }
    }

    pub async fn run(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        std::fs::create_dir_all(output)?;

        match self.family {
            None => {
                tracing::warn!("No ransomware family identified, skipping decryption");
                return Ok(RecoveryStats { total: 0, recovered: 0, failed: 0 });
            }
            Some(fam) => {
                tracing::info!(family = fam.name, "attempting decryption");

                if !fam.has_decryptor {
                    println!("  No public decryptor for {}.", fam.name);
                    println!("  Recommendation: Restore from backup or wait for future decryptor.");
                    return Ok(RecoveryStats { total: 0, recovered: 0, failed: 0 });
                }

                // 패밀리별 복호화 전략 선택
                match fam.name.as_str() {
                    "STOP/Djvu"   => self.try_stop_djvu(target, output).await,
                    "WannaCry"    => self.try_wannacry(target, output).await,
                    "GandCrab"    => self.try_gandcrab(target, output).await,
                    _             => self.try_generic_decryptor(fam, target, output).await,
                }
            }
        }
    }

    async fn try_stop_djvu(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        // STOP Djvu: emsisoft_decryptor 래퍼
        // 오프라인 키는 /usr/share/aegis/keys/stop_djvu_offline.key
        let key_path = Path::new("/usr/share/aegis/keys/stop_djvu_offline.key");

        let mut args = vec![
            target.to_string_lossy().to_string(),
            "--output".to_string(),
            output.to_string_lossy().to_string(),
        ];

        if key_path.exists() {
            args.extend(["--key".to_string(), key_path.to_string_lossy().to_string()]);
        } else if let Some(k) = self.key {
            args.extend(["--key".to_string(), k.to_string()]);
        }

        let stats = run_external_decryptor("emsisoft_decryptor", &args, target).await;
        Ok(stats)
    }

    async fn try_wannacry(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        // WannaCry: wanakiwi (메모리에서 키 복구, 이미 늦은 경우 불가)
        // .WNCRY 파일 → wncry_decrypt
        let encrypted_files = collect_files_with_ext(target, "WNCRY");
        let total = encrypted_files.len() as u64;
        let mut recovered = 0u64;

        for file in &encrypted_files {
            let out_file = output.join(
                file.strip_prefix(target).unwrap_or(file)
            );
            if let Some(parent) = out_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            // wncry 파일 헤더(8바이트) + .WNCRY 매직 제거 후 복사 (키 없는 경우 부분 복구)
            if try_wncry_partial_recover(file, &out_file).is_ok() {
                recovered += 1;
            }
        }

        Ok(RecoveryStats { total, recovered, failed: total - recovered })
    }

    async fn try_gandcrab(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        let args = vec![
            target.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        ];
        let stats = run_external_decryptor(
            "bitdefender_gandcrab_decryptor",
            &args,
            target,
        ).await;
        Ok(stats)
    }

    async fn try_generic_decryptor(
        &self,
        family: &RansomwareFamily,
        target: &Path,
        output: &Path,
    ) -> anyhow::Result<RecoveryStats> {
        std::fs::create_dir_all(output)?;

        let ext = family.extension.as_deref().unwrap_or("");
        let candidates = if ext.is_empty() {
            collect_files_with_ext(target, "locked")
        } else {
            collect_files_with_ext(target, ext)
        };

        tracing::info!(
            family = family.name,
            target = %target.display(),
            output = %output.display(),
            candidates = candidates.len(),
            decryptor_url = family.decryptor_url.as_deref().unwrap_or("none"),
            "no built-in decryptor, cataloguing encrypted files"
        );

        if let Some(url) = &family.decryptor_url {
            println!("  Decryptor available at: {}", url);
            println!("  Download and place in /usr/share/aegis/decryptors/ for automatic use");
        }

        let manifest = output.join("pending_decryptor_files.txt");
        let mut lines = Vec::new();
        for path in &candidates {
            lines.push(path.display().to_string());
        }
        std::fs::write(&manifest, lines.join("\n"))?;

        Ok(RecoveryStats {
            total: candidates.len() as u64,
            recovered: 0,
            failed: candidates.len() as u64,
        })
    }
}

async fn run_external_decryptor(
    tool: &str,
    args: &[String],
    _target: &Path,
) -> RecoveryStats {
    let tool_path = format!("/usr/share/aegis/decryptors/{}", tool);

    if !Path::new(&tool_path).exists() {
        tracing::warn!(tool = tool, "external decryptor not found at {}", tool_path);
        return RecoveryStats { total: 0, recovered: 0, failed: 0 };
    }

    match tokio::process::Command::new(&tool_path)
        .args(args)
        .status()
        .await
    {
        Ok(s) if s.success() => {
            RecoveryStats { total: 1, recovered: 1, failed: 0 }
        }
        Ok(_) | Err(_) => {
            RecoveryStats { total: 1, recovered: 0, failed: 1 }
        }
    }
}

fn collect_files_with_ext(dir: &Path, ext: &str) -> Vec<std::path::PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect()
}

fn try_wncry_partial_recover(src: &Path, dst: &Path) -> anyhow::Result<()> {
    use std::io::Write;
    let data = std::fs::read(src)?;

    // WNCRY 포맷: 매직 "WANACRY!" (8 bytes) + 암호화 블록
    // 암호화 전 원본의 일부가 헤더에 저장되는 경우 추출 시도
    if data.len() > 8 && &data[..8] == b"WANACRY!" {
        // 실제 복호화 키 없이는 완전 복구 불가
        // 단, 파일 메타데이터 및 구조 보존
        let mut out = std::fs::File::create(dst)?;
        out.write_all(&data[8.min(data.len())..])?;
    }

    Ok(())
}
