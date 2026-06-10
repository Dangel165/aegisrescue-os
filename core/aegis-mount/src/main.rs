//! aegis-mount — 안전 마운트 관리자
//!
//! 감염 디스크를 항상 RO+noexec+nodev+nosuid로 마운트하고,
//! 모든 마운트 행위를 타임스탬프+해시와 함께 로깅한다.

mod blockdev;
mod detector;
mod fstype;
mod manager;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-mount", version, about = "Safe disk mount manager")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    /// 로그 레벨
    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 디스크/파티션을 안전하게 마운트
    Mount {
        /// 장치 경로 (예: /dev/sdb1)
        device: PathBuf,
        /// 마운트 포인트 (기본: /mnt/targets/<uuid>)
        #[arg(long)]
        target: Option<PathBuf>,
        /// 읽기/쓰기 허용 (기본: 읽기 전용)
        #[arg(long)]
        rw: bool,
        /// COW 스냅샷 레이어 사용
        #[arg(long)]
        snapshot: bool,
    },
    /// 마운트 해제
    Umount {
        /// 마운트 포인트 또는 장치
        target: PathBuf,
    },
    /// 마운트된 디스크 목록 표시
    List,
    /// 감염 가능성 있는 디스크 탐지
    Detect {
        device: PathBuf,
    },
    /// 모든 블록 장치 탐색
    Scan,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    let mgr = manager::MountManager::new();

    match cli.cmd {
        Commands::Mount { device, target, rw, snapshot } => {
            let opts = aegis_common::types::MountOptions {
                read_only: !rw,
                noexec: true,
                nodev: true,
                nosuid: true,
                use_snapshot: snapshot,
            };

            if rw {
                tracing::warn!("⚠ Read-write mount requested — this is NOT safe for infected disks!");
            }

            let mount_info = mgr.mount(&device, target.as_deref(), opts).await?;
            println!("✓ Mounted: {} → {}", device.display(), mount_info.target.display());
            println!("  FS type: {:?}", mount_info.filesystem);
            if let Some(h) = &mount_info.pre_mount_hash {
                println!("  Pre-mount hash (SHA-256): {}", h);
            }
        }

        Commands::Umount { target } => {
            mgr.umount(&target).await?;
            println!("✓ Unmounted: {}", target.display());
        }

        Commands::List => {
            let mounts = mgr.list_mounts()?;
            if mounts.is_empty() {
                println!("No aegis-managed mounts.");
            } else {
                println!("{:<40} {:<20} {:<10} {}", "Source", "Target", "FS", "RO");
                for m in mounts {
                    println!(
                        "{:<40} {:<20} {:<10} {}",
                        m.source.display(),
                        m.target.display(),
                        format!("{:?}", m.filesystem),
                        if m.options.read_only { "✓" } else { "✗ RW" }
                    );
                }
            }
        }

        Commands::Detect { device } => {
            let result = detector::detect_infection_indicators(&device).await;
            println!("Infection indicators for {}:", device.display());
            for indicator in &result {
                println!("  [{}] {}", indicator.severity, indicator.description);
            }
            if result.is_empty() {
                println!("  No obvious indicators found (deep scan recommended)");
            }
        }

        Commands::Scan => {
            let devices = blockdev::list_block_devices()?;
            println!("Found {} block devices:", devices.len());
            for d in devices {
                println!("  {} ({}) - {}", d.device.display(),
                    aegis_common::utils::human_bytes(d.size_bytes),
                    format!("{:?}", d.partition_table));
            }
        }
    }

    Ok(())
}
