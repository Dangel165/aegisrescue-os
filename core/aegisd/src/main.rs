//! aegisd — AegisRescue 시스템 데몬
//!
//! 역할:
//!   - 보안 정책 적용 (polkit 게이트)
//!   - 작업 오케스트레이션 (스캔/복구/마운트 등)
//!   - Unix 소켓 IPC 서버 (GUI/CLI와 통신)
//!   - 이벤트 로그 관리
//!   - 격리 저장소 관리

mod config;
mod db;
mod job;
mod network;
mod quarantine;
mod server;
mod state;

use aegis_common::policy::AegisPolicy;
use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "aegisd", version, about = "AegisRescue system daemon")]
struct Args {
    /// 정책 파일 경로
    #[arg(long, default_value = "/etc/aegis/policy.toml")]
    policy: PathBuf,

    /// 소켓 경로 (기본: /run/aegis/aegisd.sock)
    #[arg(long, default_value = aegis_common::ipc::AEGIS_SOCKET_PATH)]
    socket: PathBuf,

    /// 데이터베이스 경로
    #[arg(long, default_value = "/var/lib/aegis/aegisd.db")]
    db: PathBuf,

    /// 로그 레벨
    #[arg(long, default_value = "info", env = "AEGIS_LOG")]
    log_level: String,

    /// 포그라운드 실행 (데몬화 안 함)
    #[arg(long)]
    foreground: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 로깅 초기화
    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .with_target(true)
        .json()
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "aegisd starting");

    // 정책 로드
    let policy = AegisPolicy::load_or_default(&args.policy);
    info!(network_mode = ?policy.network.default, "policy loaded");

    // 소켓 디렉터리 생성
    if let Some(parent) = args.socket.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 데이터베이스 디렉터리 생성
    if let Some(parent) = args.db.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 공유 상태 초기화
    let state = state::AppState::new(policy, args.db).await?;

    // 네트워크 격리 정책 적용
    network::apply_policy(&state.policy().network).await;

    // SIGTERM / SIGINT 처리
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate()
        ).expect("SIGTERM handler");
        sigterm.recv().await;
        info!("SIGTERM received, shutting down");
        state_clone.shutdown().await;
        std::process::exit(0);
    });

    // IPC 서버 실행
    info!(socket = %args.socket.display(), "starting IPC server");
    if let Err(e) = server::run(state, &args.socket).await {
        error!("IPC server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
