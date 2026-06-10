//! 파일 카빙 — PhotoRec/TestDisk 래퍼 + 자체 시그니처 카버

use std::path::Path;

pub struct CarvingStats {
    pub total_carved: u64,
}

/// 카빙 대상 파일 타입 시그니처
struct FileSignature {
    name: &'static str,
    header: &'static [u8],
    footer: Option<&'static [u8]>,
    extensions: &'static [&'static str],
    max_size: usize,
}

static SIGNATURES: &[FileSignature] = &[
    FileSignature {
        name: "JPEG",
        header: b"\xFF\xD8\xFF",
        footer: Some(b"\xFF\xD9"),
        extensions: &["jpg"],
        max_size: 20 * 1024 * 1024,
    },
    FileSignature {
        name: "PNG",
        header: b"\x89PNG\r\n\x1A\n",
        footer: Some(b"IEND\xAEB`\x82"),
        extensions: &["png"],
        max_size: 20 * 1024 * 1024,
    },
    FileSignature {
        name: "PDF",
        header: b"%PDF",
        footer: Some(b"%%EOF"),
        extensions: &["pdf"],
        max_size: 100 * 1024 * 1024,
    },
    FileSignature {
        name: "ZIP/DOCX/XLSX",
        header: b"PK\x03\x04",
        footer: None,
        extensions: &["zip", "docx", "xlsx"],
        max_size: 200 * 1024 * 1024,
    },
    FileSignature {
        name: "Windows PE",
        header: b"MZ",
        footer: None,
        extensions: &["exe"],
        max_size: 100 * 1024 * 1024,
    },
    FileSignature {
        name: "SQLite",
        header: b"SQLite format 3",
        footer: None,
        extensions: &["db", "sqlite"],
        max_size: 500 * 1024 * 1024,
    },
];

pub struct FileCarver {
    target_types: Vec<String>,
}

impl FileCarver {
    pub fn new(types: &[String]) -> Self {
        Self {
            target_types: types.to_vec(),
        }
    }

    pub async fn carve(&self, device: &Path, output: &Path) -> anyhow::Result<CarvingStats> {
        std::fs::create_dir_all(output)?;

        // 1) PhotoRec 사용 가능 시 우선 사용
        if let Ok(stats) = self.run_photorec(device, output).await {
            return Ok(stats);
        }

        // 2) 자체 카버 폴백
        self.run_native_carver(device, output).await
    }

    async fn run_photorec(&self, device: &Path, output: &Path) -> anyhow::Result<CarvingStats> {
        // photorec CLI: non-interactive mode
        let status = tokio::process::Command::new("photorec")
            .args([
                "/d", &output.to_string_lossy(),
                "/cmd", &device.to_string_lossy(),
                "fileopt,enable,everything,freespace,search",
            ])
            .status()
            .await?;

        if status.success() {
            // 복구 파일 수 계산
            let count = walkdir::WalkDir::new(output)
                .into_iter().filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count() as u64;

            tracing::info!(carved = count, "photorec carving complete");
            Ok(CarvingStats { total_carved: count })
        } else {
            Err(anyhow::anyhow!("photorec failed"))
        }
    }

    async fn run_native_carver(
        &self,
        device: &Path,
        output: &Path,
    ) -> anyhow::Result<CarvingStats> {
        tracing::info!("Running native file carver");

        let data = std::fs::read(device)?;
        let mut carved = 0u64;

        let sigs: Vec<&FileSignature> = if self.target_types.is_empty() {
            SIGNATURES.iter().collect()
        } else {
            SIGNATURES.iter()
                .filter(|s| self.target_types.iter().any(|t| s.extensions.contains(&t.as_str())))
                .collect()
        };

        for sig in sigs {
            let found = carve_signature(&data, sig, output)?;
            carved += found;
        }

        tracing::info!(carved = carved, "native carver complete");
        Ok(CarvingStats { total_carved: carved })
    }
}

fn carve_signature(
    data: &[u8],
    sig: &FileSignature,
    output: &Path,
) -> anyhow::Result<u64> {
    let mut count = 0u64;
    let mut offset = 0usize;

    while offset < data.len() {
        // 헤더 탐색
        if let Some(pos) = find_bytes(&data[offset..], sig.header) {
            let start = offset + pos;

            // 최대 크기 또는 푸터로 끝 결정
            let end = if let Some(footer) = sig.footer {
                find_bytes(&data[start..], footer)
                    .map(|p| (start + p + footer.len()).min(start + sig.max_size))
                    .unwrap_or((start + sig.max_size).min(data.len()))
            } else {
                (start + sig.max_size).min(data.len())
            };

            if end > start + 64 { // 최소 크기 확인
                let filename = format!("{}_offset_{:08x}.{}",
                    sig.name.replace('/', "_"), start, sig.extensions[0]);
                let dest = output.join(&filename);

                std::fs::write(&dest, &data[start..end])?;
                count += 1;
            }

            offset = start + sig.header.len();
        } else {
            break;
        }
    }

    Ok(count)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
