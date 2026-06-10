//! aegis-sandbox — 샌드박스 런처
//!
//! 격리 레벨:
//!   L0: 정적 분석만 (실행 없음)
//!   L1: 네임스페이스 격리 (user+mount+net+pid+ipc + seccomp)
//!   L2: microVM (QEMU/Firecracker KVM)

mod namespace;
mod seccomp;
mod vm;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-sandbox", version, about = "Isolated sample analysis sandbox")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 샘플 분석 실행
    Run {
        /// 분석 대상 파일 (읽기 전용 주입)
        sample: PathBuf,
        /// 격리 레벨 (0=정적, 1=네임스페이스, 2=VM)
        #[arg(long, default_value = "1")]
        level: u8,
        /// 분석 타임아웃 (초)
        #[arg(long, default_value = "60")]
        timeout: u64,
        /// 분석 결과 출력 경로
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// 샌드박스 상태 확인
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    match cli.cmd {
        Commands::Run { sample, level, timeout, output } => {
            println!("AegisRescue Sandbox — Level {} isolation", level);
            println!("Sample: {}", sample.display());
            println!("Timeout: {}s", timeout);

            let result = match level {
                0 => {
                    println!("Mode: Static analysis only");
                    analyze_static(&sample).await
                }
                1 => {
                    println!("Mode: Namespace isolation (L1)");
                    namespace::run_in_namespace(&sample, timeout).await
                }
                2 => {
                    println!("Mode: microVM isolation (L2)");
                    vm::run_in_vm(&sample, timeout).await
                }
                _ => Err(anyhow::anyhow!("Invalid isolation level: {}", level)),
            }?;

            if let Some(out) = output {
                std::fs::write(&out, serde_json::to_string_pretty(&result)?)?;
                println!("Results written to {}", out.display());
            } else {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Commands::Status => {
            println!("Sandbox capabilities:");
            println!("  Namespace isolation: {}", if check_namespace_support() { "✓" } else { "✗" });
            println!("  KVM/microVM:         {}", if check_kvm_support() { "✓" } else { "✗ (no KVM)" });
            println!("  seccomp-bpf:         {}", if check_seccomp_support() { "✓" } else { "✗" });
        }
    }

    Ok(())
}

async fn analyze_static(sample: &std::path::Path) -> anyhow::Result<serde_json::Value> {
    let features = aegis_ai::features::FeatureVector::extract(sample)?;
    let classifier = aegis_ai::AiClassifier::new(
        std::path::Path::new("/usr/share/aegis/models")
    );
    let ai_result = classifier.classify(&features);

    Ok(serde_json::json!({
        "mode": "static",
        "sample": sample.display().to_string(),
        "ai_classification": ai_result,
        "entropy": features.entropy,
    }))
}

fn check_namespace_support() -> bool {
    std::path::Path::new("/proc/self/ns/user").exists()
}

fn check_kvm_support() -> bool {
    std::path::Path::new("/dev/kvm").exists()
}

fn check_seccomp_support() -> bool {
    // Linux 3.5+ 필요
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/proc/sys/kernel/seccomp").exists()
    }
    #[cfg(not(target_os = "linux"))]
    { false }
}
