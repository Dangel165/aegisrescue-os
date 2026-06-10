//! aegis-recover — 랜섬웨어 및 파일 복구 엔진

mod carver;
mod decryptor;
mod family;
mod shadow;
mod verifier;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-recover", version, about = "Ransomware & file recovery engine")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 랜섬웨어 패밀리 식별
    Identify {
        /// 대상 디스크/파티션
        target: PathBuf,
    },
    /// 알려진 복호화기로 복구 시도
    Decrypt {
        target: PathBuf,
        /// 출력 경로 (원본은 절대 덮어쓰지 않음)
        #[arg(long, default_value = "/mnt/evidence/recovered")]
        output: PathBuf,
        /// 복호화 키 (알고 있는 경우)
        #[arg(long)]
        key: Option<String>,
    },
    /// Windows Volume Shadow Copy에서 복원
    Vss {
        target: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/recovered")]
        output: PathBuf,
    },
    /// 파일 카빙 (삭제/손상 파일 복구)
    Carve {
        /// 디스크 이미지 또는 파티션
        device: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/carved")]
        output: PathBuf,
        /// 복구 대상 파일 타입 (기본: 모두)
        #[arg(long)]
        types: Vec<String>,
    },
    /// 모든 복구 방법 순차 시도
    Auto {
        target: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/recovered")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    match cli.cmd {
        Commands::Identify { target } => {
            let result = family::identify_ransomware(&target);
            match result {
                Some(fam) => {
                    println!("Ransomware family: {}", fam.name);
                    println!("Extension:         {}", fam.extension.as_deref().unwrap_or("unknown"));
                    println!("Decryptor:         {}", if fam.has_decryptor { "✓ Available" } else { "✗ Not available" });
                    println!("Notes:             {}", fam.notes);
                }
                None => {
                    println!("Unknown ransomware family (or not ransomware)");
                    println!("Run 'aegis-scan --full' for deeper analysis");
                }
            }
        }

        Commands::Decrypt { target, output, key } => {
            let fam = family::identify_ransomware(&target);
            let decryptor = decryptor::Decryptor::new(fam.as_ref(), key.as_deref());
            let stats = decryptor.run(&target, &output).await?;
            println!("Recovery complete: {}/{} files recovered", stats.recovered, stats.total);
        }

        Commands::Vss { target, output } => {
            let stats = shadow::recover_from_vss(&target, &output).await?;
            println!("VSS recovery: {}/{} files restored", stats.recovered, stats.total);
        }

        Commands::Carve { device, output, types } => {
            let carver = carver::FileCarver::new(&types);
            let stats = carver.carve(&device, &output).await?;
            println!("Carving complete: {} files recovered", stats.total_carved);
        }

        Commands::Auto { target, output } => {
            println!("=== AegisRescue Auto Recovery ===");
            println!();

            // 1단계: 패밀리 식별
            println!("[1/4] Identifying ransomware family...");
            let fam = family::identify_ransomware(&target);
            if let Some(ref f) = fam {
                println!("  Found: {} (decryptor: {})", f.name, if f.has_decryptor { "yes" } else { "no" });
            } else {
                println!("  Unknown family");
            }

            // 2단계: 알려진 복호화기
            println!("[2/4] Trying known decryptors...");
            let dec = decryptor::Decryptor::new(fam.as_ref(), None);
            let dec_stats = dec.run(&target, &output).await?;
            println!("  Recovered: {}/{}", dec_stats.recovered, dec_stats.total);

            // 3단계: VSS 복원
            println!("[3/4] Attempting VSS/snapshot recovery...");
            match shadow::recover_from_vss(&target, &output).await {
                Ok(s) => println!("  VSS: {}/{} restored", s.recovered, s.total),
                Err(e) => println!("  VSS: not available ({})", e),
            }

            // 4단계: 파일 카빙
            println!("[4/4] File carving...");
            let carve_output = output.join("carved");
            let carver = carver::FileCarver::new(&[]);
            match carver.carve(&target, &carve_output).await {
                Ok(s) => println!("  Carved: {} files", s.total_carved),
                Err(e) => println!("  Carving: failed ({})", e),
            }

            println!();
            println!("✓ Auto recovery completed. Output: {}", output.display());
        }
    }

    Ok(())
}
