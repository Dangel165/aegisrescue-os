//! aegis-image — 디스크 이미징 및 백업

mod clone;
mod ddrescue;
mod restore;
mod verify;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-image", version, about = "Disk imaging, cloning and backup")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 디스크 이미지 생성
    Create {
        source: PathBuf,
        dest: PathBuf,
        /// zstd 압축
        #[arg(long)]
        compress: bool,
        /// 손상 디스크 (ddrescue 모드)
        #[arg(long)]
        damaged: bool,
    },
    /// 이미지 → 디스크 복원
    Restore {
        image: PathBuf,
        dest: PathBuf,
        /// 강제 (확인 생략)
        #[arg(long, short)]
        force: bool,
    },
    /// 디스크 → 디스크 클론
    Clone {
        source: PathBuf,
        dest: PathBuf,
    },
    /// 이미지 무결성 검증
    Verify {
        image: PathBuf,
        #[arg(long)]
        hash: Option<String>,
    },
    /// 파일/디렉터리 백업 (restic/borg 래퍼)
    Backup {
        source: PathBuf,
        dest: PathBuf,
        /// 증분 백업
        #[arg(long)]
        incremental: bool,
        /// 암호화 패스프레이즈
        #[arg(long, env = "AEGIS_BACKUP_PASSPHRASE")]
        passphrase: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    match cli.cmd {
        Commands::Create { source, dest, compress, damaged } => {
            if damaged {
                println!("Using ddrescue mode for damaged disk...");
                let stats = ddrescue::image_damaged_disk(&source, &dest).await?;
                println!("✓ Rescued: {} sectors ({} errors, {} bytes)",
                    stats.rescued_sectors, stats.error_sectors, stats.size_bytes);
            } else {
                let image_path = clone::create_image(&source, &dest, compress).await?;
                println!("✓ Image created: {}", image_path.display());
            }
        }

        Commands::Restore { image, dest, force } => {
            if !force {
                print!("⚠ This will overwrite {}. Type 'YES' to continue: ", dest.display());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim() != "YES" {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            restore::restore_image(&image, &dest).await?;
            println!("✓ Restored to {}", dest.display());
        }

        Commands::Clone { source, dest } => {
            clone::clone_disk(&source, &dest).await?;
            println!("✓ Cloned {} → {}", source.display(), dest.display());
        }

        Commands::Verify { image, hash } => {
            let result = verify::verify_image(&image, hash.as_deref())?;
            if result.valid {
                println!("✓ Image is valid");
                println!("  SHA-256: {}", result.sha256);
            } else {
                println!("✗ Image verification FAILED");
                std::process::exit(1);
            }
        }

        Commands::Backup { source, dest, incremental, passphrase } => {
            let stats = clone::backup_files(&source, &dest, incremental, passphrase.as_deref()).await?;
            println!("✓ Backup complete: {} files, {}",
                stats.files, aegis_common::utils::human_bytes(stats.bytes));
        }
    }

    Ok(())
}
