//! aegis — AegisRescue 통합 CLI
//!
//! 사용자가 직접 실행하거나 aegisd에 작업을 위임하는 단일 진입점

mod client;
mod display;

use aegis_common::ipc::{DaemonRequest, DaemonResponse};
use aegis_common::types::JobType;
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "aegis",
    version,
    about = "AegisRescue — Malware Recovery & Digital Forensics",
    long_about = "AegisRescue unified command-line interface.\n\
                  Communicates with aegisd for job management.\n\n\
                  Examples:\n  \
                  aegis scan /mnt/targets/windows\n  \
                  aegis recover auto /mnt/targets/windows\n  \
                  aegis boot repair /dev/sda"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    /// JSON 출력
    #[arg(long, global = true)]
    json: bool,

    /// 상세 로그
    #[arg(long, short, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 악성코드 스캔
    Scan {
        target: PathBuf,
        #[arg(long, short)]
        quick: bool,
        #[arg(long)]
        quarantine: bool,
    },
    /// 랜섬웨어 복구
    Recover {
        target: PathBuf,
        #[arg(long, default_value = "/mnt/evidence/recovered")]
        output: PathBuf,
    },
    /// 부트 레코드 복구
    Boot {
        #[command(subcommand)]
        action: BootAction,
    },
    /// 디스크 이미징
    Image {
        source: PathBuf,
        dest: PathBuf,
        #[arg(long)]
        compress: bool,
    },
    /// 디지털 포렌식
    Forensic {
        #[command(subcommand)]
        action: ForensicAction,
    },
    /// 디스크 목록
    Disks,
    /// 작업 목록
    Jobs,
    /// 시스템 상태
    Status,
    /// AI 분석
    Ai {
        file: PathBuf,
    },
    /// 샌드박스 분석
    Sandbox {
        sample: PathBuf,
        #[arg(long, default_value = "1")]
        level: u8,
    },
    /// 네트워크 모드 변경
    Network {
        /// airgap | whitelist | restricted | full
        mode: String,
    },
}

#[derive(Subcommand)]
enum BootAction {
    Analyze { device: PathBuf },
    Repair  { device: PathBuf, #[arg(long)] windows: bool, #[arg(long)] linux: bool },
}

#[derive(Subcommand)]
enum ForensicAction {
    Capture  { source: PathBuf, #[arg(long, default_value = "/mnt/evidence")] output: PathBuf },
    Timeline { root: PathBuf },
    Verify   { hash_file: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
    }

    print_banner();

    match &cli.cmd {
        Commands::Scan { target, quick, quarantine } => {
            let args: Vec<String> = {
                let mut a = vec![target.to_string_lossy().to_string()];
                if *quick   { a.push("--quick".into()); }
                if *quarantine { a.push("--quarantine".into()); }
                a
            };
            run_tool("aegis-scan", &args)?;
        }

        Commands::Recover { target, output } => {
            run_tool("aegis-recover", &[
                "auto".to_string(),
                target.to_string_lossy().to_string(),
                "--output".to_string(),
                output.to_string_lossy().to_string(),
            ])?;
        }

        Commands::Boot { action } => {
            match action {
                BootAction::Analyze { device } => {
                    run_tool("aegis-boot", &["analyze".to_string(), device.to_string_lossy().to_string()])?;
                }
                BootAction::Repair { device, windows, linux } => {
                    let mut args = vec!["repair".to_string(), device.to_string_lossy().to_string()];
                    if *windows { args.push("--windows".into()); }
                    if *linux   { args.push("--linux".into()); }
                    run_tool("aegis-boot", &args)?;
                }
            }
        }

        Commands::Image { source, dest, compress } => {
            let mut args = vec![
                "create".to_string(),
                source.to_string_lossy().to_string(),
                dest.to_string_lossy().to_string(),
            ];
            if *compress { args.push("--compress".into()); }
            run_tool("aegis-image", &args)?;
        }

        Commands::Forensic { action } => {
            match action {
                ForensicAction::Capture { source, output } => {
                    run_tool("aegis-forensic", &[
                        "capture".to_string(),
                        source.to_string_lossy().to_string(),
                        "--output".to_string(),
                        output.to_string_lossy().to_string(),
                    ])?;
                }
                ForensicAction::Timeline { root } => {
                    run_tool("aegis-forensic", &["timeline".to_string(), root.to_string_lossy().to_string()])?;
                }
                ForensicAction::Verify { hash_file } => {
                    run_tool("aegis-forensic", &["verify".to_string(), hash_file.to_string_lossy().to_string()])?;
                }
            }
        }

        Commands::Ai { file } => {
            run_tool("aegis-ai", &[file.to_string_lossy().to_string()])?;
        }

        Commands::Sandbox { sample, level } => {
            run_tool("aegis-sandbox", &[
                "run".to_string(),
                sample.to_string_lossy().to_string(),
                "--level".to_string(),
                level.to_string(),
            ])?;
        }

        Commands::Status => {
            display::print_system_status();
        }

        Commands::Disks => {
            display::print_disk_list();
        }

        Commands::Jobs => {
            display::print_jobs();
        }

        Commands::Network { mode } => {
            println!("Network mode change requires aegisd (daemon not connected in CLI mode)");
            println!("Run: systemctl status aegisd");
        }
    }

    Ok(())
}

fn run_tool(tool: &str, args: &[String]) -> anyhow::Result<()> {
    let status = std::process::Command::new(tool)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run {}: {}", tool, e))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn print_banner() {
    println!("{}", style("╔══════════════════════════════════════════╗").cyan());
    println!("{}", style("║  AegisRescue — Malware Recovery OS v0.1  ║").cyan());
    println!("{}", style("╚══════════════════════════════════════════╝").cyan());
}
