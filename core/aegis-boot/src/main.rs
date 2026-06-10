//! aegis-boot — 부트 레코드 복구 도구

mod backup;
mod bcd;
mod efi;
mod grub;
mod mbr;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-boot", version, about = "Boot record repair tool (MBR/GPT/EFI/GRUB/BCD)")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 부트 레코드 분석 (변조 감지)
    Analyze {
        device: PathBuf,
    },
    /// 부트 레코드 백업
    Backup {
        device: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/boot_backup")]
        output: PathBuf,
    },
    /// 부트 레코드 복구 (종합)
    Repair {
        device: PathBuf,
        /// Windows BCD 포함 복구
        #[arg(long)]
        windows: bool,
        /// Linux GRUB 복구
        #[arg(long)]
        linux: bool,
        /// EFI 부트 항목 복구
        #[arg(long)]
        efi: bool,
    },
    /// MBR 재작성
    FixMbr {
        device: PathBuf,
    },
    /// GPT 복원 (백업 GPT에서)
    FixGpt {
        device: PathBuf,
    },
    /// EFI 부트로더 재설치
    FixEfi {
        #[arg(long)]
        esp: PathBuf,
        #[arg(long)]
        os: String,
    },
    /// Windows BCD 재생성
    FixBcd {
        /// Windows 파티션 마운트 포인트
        windows_root: PathBuf,
    },
    /// GRUB 재설치
    FixGrub {
        device: PathBuf,
        /// Linux 루트 파티션 마운트 포인트
        #[arg(long)]
        root: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    match cli.cmd {
        Commands::Analyze { device } => {
            let report = mbr::analyze_boot_record(&device)?;
            println!("{}", report);
        }

        Commands::Backup { device, output } => {
            std::fs::create_dir_all(&output)?;
            let path = backup::backup_boot_record(&device, &output)?;
            println!("✓ Boot record backed up to: {}", path.display());
        }

        Commands::Repair { device, windows, linux, efi } => {
            println!("=== AegisRescue Boot Repair ===");

            // 1. 먼저 백업
            let backup_dir = PathBuf::from("/mnt/evidence/boot_backup");
            std::fs::create_dir_all(&backup_dir).ok();
            if let Ok(p) = backup::backup_boot_record(&device, &backup_dir) {
                println!("✓ Boot record backed up to {}", p.display());
            }

            // 2. MBR/GPT 분석
            let analysis = mbr::analyze_boot_record(&device)?;
            println!("{}", analysis);

            // 3. 대상별 복구
            if windows {
                bcd::repair_windows_boot(&device).await?;
            }
            if linux {
                // root 파티션 자동 탐지
                let root = PathBuf::from("/mnt/targets/linux_root");
                grub::repair_grub(&device, &root).await?;
            }
            if efi {
                let esp = PathBuf::from("/mnt/targets/esp");
                efi::repair_efi(&esp).await?;
            }

            // 기본: MBR/GPT 복구만
            if !windows && !linux && !efi {
                mbr::repair_mbr(&device)?;
            }

            println!("✓ Boot repair completed");
        }

        Commands::FixMbr { device } => {
            backup::backup_boot_record(&device, &PathBuf::from("/mnt/evidence/boot_backup")).ok();
            mbr::repair_mbr(&device)?;
            println!("✓ MBR repaired on {}", device.display());
        }

        Commands::FixGpt { device } => {
            backup::backup_boot_record(&device, &PathBuf::from("/mnt/evidence/boot_backup")).ok();
            mbr::repair_gpt(&device)?;
            println!("✓ GPT repaired on {}", device.display());
        }

        Commands::FixEfi { esp, os } => {
            efi::install_bootloader(&esp, &os).await?;
            println!("✓ EFI bootloader installed for {}", os);
        }

        Commands::FixBcd { windows_root } => {
            bcd::repair_windows_boot_from_root(&windows_root).await?;
            println!("✓ Windows BCD repaired");
        }

        Commands::FixGrub { device, root } => {
            grub::repair_grub(&device, &root).await?;
            println!("✓ GRUB repaired on {}", device.display());
        }
    }

    Ok(())
}
