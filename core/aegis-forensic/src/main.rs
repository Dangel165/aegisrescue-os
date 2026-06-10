//! aegis-forensic — 디지털 포렌식 모듈

mod artifacts;
mod custody;
mod imager;
mod timeline;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-forensic", version, about = "Digital forensics module")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 증거 수집 (해시 체인 보장)
    Capture {
        /// 증거 소스 (디스크 또는 디렉터리)
        source: PathBuf,
        /// 출력 경로
        #[arg(long, default_value = "/mnt/evidence")]
        output: PathBuf,
    },
    /// 타임라인 분석
    Timeline {
        root: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/timeline.jsonl")]
        output: PathBuf,
    },
    /// 아티팩트 수집 (레지스트리, 이벤트로그, 브라우저 등)
    Artifacts {
        root: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/artifacts")]
        output: PathBuf,
    },
    /// 해시 검증
    Verify {
        /// 해시 파일 (.sha256)
        hash_file: PathBuf,
    },
    /// 체인 오브 커스터디 리포트
    Custody {
        #[arg(long, default_value = "/mnt/evidence")]
        evidence_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    match cli.cmd {
        Commands::Capture { source, output } => {
            let result = imager::capture_evidence(&source, &output).await?;
            println!("✓ Evidence captured");
            println!("  SHA-256: {}", result.sha256);
            println!("  BLAKE3:  {}", result.blake3);
            println!("  Size:    {}", aegis_common::utils::human_bytes(result.size_bytes));
        }

        Commands::Timeline { root, output } => {
            timeline::build_timeline(&root, &output).await?;
            println!("✓ Timeline written to {}", output.display());
        }

        Commands::Artifacts { root, output } => {
            artifacts::collect_all(&root, &output).await?;
            println!("✓ Artifacts collected to {}", output.display());
        }

        Commands::Verify { hash_file } => {
            let results = imager::verify_hashes(&hash_file)?;
            let mut ok = 0usize;
            let mut fail = 0usize;
            for (path, verified) in &results {
                if *verified {
                    println!("✓ {}", path);
                    ok += 1;
                } else {
                    println!("✗ {} — HASH MISMATCH", path);
                    fail += 1;
                }
            }
            println!("\n{}/{} files verified", ok, ok + fail);
            if fail > 0 {
                std::process::exit(1);
            }
        }

        Commands::Custody { evidence_dir } => {
            let report = custody::generate_report(&evidence_dir)?;
            println!("{}", report);
        }
    }

    Ok(())
}
