//! 알려진 복호화 도구 실행 관리자

use crate::family::RansomwareFamily;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DECRYPTOR_DIR: &str = "/usr/share/aegis/decryptors";
const MANIFEST_PATH: &str = "/usr/share/aegis/decryptors/manifest.toml";

pub struct RecoveryStats {
    pub total: u64,
    pub recovered: u64,
    pub failed: u64,
}

pub struct Decryptor<'a> {
    family: Option<&'a RansomwareFamily>,
    key: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    tools: HashMap<String, ToolEntry>,
    #[serde(default)]
    families: HashMap<String, FamilyEntry>,
}

#[derive(Debug, Deserialize)]
struct ToolEntry {
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FamilyEntry {
    tool: String,
    #[serde(default)]
    #[allow(dead_code)]
    extension: String,
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
                Ok(RecoveryStats {
                    total: 0,
                    recovered: 0,
                    failed: 0,
                })
            }
            Some(fam) => {
                tracing::info!(family = fam.name, "attempting decryption");

                if !fam.has_decryptor {
                    println!("  No public decryptor for {}.", fam.name);
                    println!(
                        "  Recommendation: Restore from backup or wait for future decryptor."
                    );
                    return Ok(RecoveryStats {
                        total: 0,
                        recovered: 0,
                        failed: 0,
                    });
                }

                if let Some(stats) = self.try_manifest(fam, target, output).await? {
                    return Ok(stats);
                }

                match fam.name.as_str() {
                    "STOP/Djvu" => self.try_stop_djvu(target, output).await,
                    "WannaCry" => self.try_wannacry(target, output).await,
                    "GandCrab" => self.try_gandcrab(target, output).await,
                    _ => self.try_generic_decryptor(fam, target, output).await,
                }
            }
        }
    }

    async fn try_manifest(
        &self,
        family: &RansomwareFamily,
        target: &Path,
        output: &Path,
    ) -> anyhow::Result<Option<RecoveryStats>> {
        let manifest_path = Path::new(MANIFEST_PATH);
        if !manifest_path.exists() {
            return Ok(None);
        }

        let text = std::fs::read_to_string(manifest_path)?;
        let manifest: Manifest = toml::from_str(&text)?;

        let family_entry = match manifest.families.get(&family.name) {
            Some(e) => e,
            None => return Ok(None),
        };
        let tool_name = family_entry.tool.clone();

        let tool_entry = manifest.tools.get(&tool_name);
        let args_template = tool_entry
            .map(|t| t.args.clone())
            .unwrap_or_else(|| vec![
                "{target}".into(),
                "{output}".into(),
                "{family}".into(),
            ]);

        let args: Vec<String> = args_template
            .iter()
            .map(|a| {
                a.replace("{target}", &target.to_string_lossy())
                    .replace("{output}", &output.to_string_lossy())
                    .replace("{family}", &family.name)
                    .replace("{key}", self.key.unwrap_or(""))
            })
            .collect();

        let stats = run_external_decryptor(&tool_name, &args, target).await;
        Ok(Some(stats))
    }

    async fn try_stop_djvu(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        let key_path = Path::new("/usr/share/aegis/keys/stop_djvu_offline.key");

        let mut args = vec![
            target.to_string_lossy().to_string(),
            "--output".to_string(),
            output.to_string_lossy().to_string(),
        ];

        if key_path.exists() {
            args.extend([
                "--key".to_string(),
                key_path.to_string_lossy().to_string(),
            ]);
        } else if let Some(k) = self.key {
            args.extend(["--key".to_string(), k.to_string()]);
        }

        let stats = run_external_decryptor("emsisoft_decryptor", &args, target).await;
        Ok(stats)
    }

    async fn try_wannacry(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        let encrypted_files = collect_files_with_ext(target, "WNCRY");
        let total = encrypted_files.len() as u64;
        let mut recovered = 0u64;

        for file in &encrypted_files {
            let out_file = output.join(file.strip_prefix(target).unwrap_or(file));
            if let Some(parent) = out_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            if try_wncry_partial_recover(file, &out_file).is_ok() {
                recovered += 1;
            }
        }

        Ok(RecoveryStats {
            total,
            recovered,
            failed: total - recovered,
        })
    }

    async fn try_gandcrab(&self, target: &Path, output: &Path) -> anyhow::Result<RecoveryStats> {
        let args = vec![
            target.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
        ];
        let stats = run_external_decryptor("bitdefender_gandcrab_decryptor", &args, target).await;
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
            "running generic decryptor catalog"
        );

        if let Some(url) = &family.decryptor_url {
            println!("  Decryptor available at: {}", url);
            println!("  Download vendor binary to /usr/share/aegis/decryptors/vendor/");
        }

        let args = vec![
            target.to_string_lossy().into_owned(),
            output.to_string_lossy().into_owned(),
            family.name.clone(),
        ];
        let stats = run_external_decryptor("aegis_generic_decryptor", &args, target).await;

        if stats.total == 0 && !candidates.is_empty() {
            let manifest = output.join("pending_decryptor_files.txt");
            let lines: Vec<String> = candidates.iter().map(|p| p.display().to_string()).collect();
            std::fs::write(&manifest, lines.join("\n"))?;
            return Ok(RecoveryStats {
                total: candidates.len() as u64,
                recovered: 0,
                failed: candidates.len() as u64,
            });
        }

        Ok(stats)
    }
}

async fn run_external_decryptor(tool: &str, args: &[String], target: &Path) -> RecoveryStats {
    let tool_path = PathBuf::from(DECRYPTOR_DIR).join(tool);

    if !tool_path.exists() {
        tracing::warn!(tool = tool, path = %tool_path.display(), "external decryptor not found");
        return RecoveryStats {
            total: 0,
            recovered: 0,
            failed: 0,
        };
    }

    let total = count_target_files(target);

    match tokio::process::Command::new(&tool_path)
        .args(args)
        .status()
        .await
    {
        Ok(s) if s.success() => RecoveryStats {
            total: total.max(1),
            recovered: total.max(1),
            failed: 0,
        },
        Ok(_) => RecoveryStats {
            total: total.max(1),
            recovered: 0,
            failed: total.max(1),
        },
        Err(e) => {
            tracing::warn!(tool = tool, error = %e, "decryptor execution failed");
            RecoveryStats {
                total: total.max(1),
                recovered: 0,
                failed: total.max(1),
            }
        }
    }
}

fn count_target_files(target: &Path) -> u64 {
    walkdir::WalkDir::new(target)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u64
}

fn collect_files_with_ext(dir: &Path, ext: &str) -> Vec<std::path::PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
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

    if data.len() > 8 && &data[..8] == b"WANACRY!" {
        let mut out = std::fs::File::create(dst)?;
        out.write_all(&data[8.min(data.len())..])?;
    }

    Ok(())
}
