//! aegis-scan — 멀티엔진 악성코드 스캐너

mod ai_engine;
mod engines;
mod orchestrator;
mod persistence;
mod pe_analyzer;
mod quarantine_client;
mod report;

use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "aegis-scan", version, about = "Multi-engine malware scanner")]
struct Cli {
    /// 스캔 대상 경로 (파일 또는 디렉터리)
    target: PathBuf,

    /// 빠른 스캔 (자동 실행 지점 + 최근 파일만)
    #[arg(long, short)]
    quick: bool,

    /// 전체 심층 스캔
    #[arg(long, short)]
    full: bool,

    /// YARA 룰 경로 (기본: /usr/share/aegis/yara/)
    #[arg(long, default_value = "/usr/share/aegis/yara")]
    yara_rules: PathBuf,

    /// 시그니처 데이터베이스 경로
    #[arg(long, default_value = "/usr/share/aegis/signatures")]
    signatures: PathBuf,

    /// 결과를 JSON으로 출력
    #[arg(long)]
    json: bool,

    /// 격리 실행 (탐지된 파일 자동 격리)
    #[arg(long)]
    quarantine: bool,

    /// 병렬 스레드 수
    #[arg(long, default_value = "4")]
    threads: usize,

    /// 로그 레벨
    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    let scan_mode = if cli.quick {
        orchestrator::ScanMode::Quick
    } else if cli.full {
        orchestrator::ScanMode::Full
    } else {
        orchestrator::ScanMode::Standard
    };

    let config = orchestrator::ScanConfig {
        target: cli.target.clone(),
        mode: scan_mode,
        yara_rules_path: cli.yara_rules,
        signatures_path: cli.signatures,
        threads: cli.threads,
        auto_quarantine: cli.quarantine,
    };

    let scanner = orchestrator::Scanner::new(config);
    let results = scanner.run().await?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        report::print_summary(&results);
    }

    // 악성 파일 발견 시 비-zero 종료 코드
    if results.malicious_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}
