//! aegis-update — AegisRescue 업데이트 시스템
//!
//! 서명 DB, YARA 룰, AI 모델, OS 컴포넌트를 업데이트합니다.
//! Air-Gap 환경에서는 오프라인 번들(.tar.zst + .sig) 방식을 지원합니다.

mod bundle;
mod channel;
mod manifest;
mod verify;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aegis-update", about = "AegisRescue update manager")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// 업데이트 채널 (stable / testing)
    #[arg(long, default_value = "stable", global = true)]
    channel: String,

    /// 데이터 디렉터리
    #[arg(long, default_value = "/var/lib/aegis", global = true)]
    data_dir: PathBuf,

    /// 서명 키 경로 (공개 키)
    #[arg(long, default_value = "/etc/aegis/keys/update.pub", global = true)]
    pub_key: PathBuf,
}

#[derive(Subcommand)]
enum Cmd {
    /// 사용 가능한 업데이트 확인
    Check,
    /// 업데이트 적용 (네트워크 또는 번들)
    Apply {
        /// 오프라인 번들 파일 경로 (.tar.zst)
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// 업데이트 항목 선택 (기본: 전체)
        #[arg(long, value_delimiter = ',')]
        only: Vec<UpdateTarget>,
        /// 실제 적용 없이 시뮬레이션만
        #[arg(long)]
        dry_run: bool,
    },
    /// 번들 파일 생성 (에어갭 환경으로 내보내기)
    Bundle {
        /// 출력 경로
        #[arg(short, long)]
        output: PathBuf,
        /// 번들에 포함할 항목
        #[arg(long, value_delimiter = ',')]
        include: Vec<UpdateTarget>,
    },
    /// 현재 컴포넌트 버전 표시
    Status,
    /// 특정 버전으로 롤백
    Rollback {
        /// 롤백 대상 컴포넌트
        target: UpdateTarget,
        /// 롤백할 버전 (기본: 직전 버전)
        #[arg(long)]
        version: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum UpdateTarget {
    #[value(name = "sigs")]      Signatures,
    #[value(name = "yara")]      YaraRules,
    #[value(name = "ioc")]       IocDatabase,
    #[value(name = "ai-model")]  AiModel,
    #[value(name = "decryptors")] Decryptors,
    #[value(name = "core")]      CoreBinaries,
    #[value(name = "policy")]    SecurityPolicy,
}

impl UpdateTarget {
    pub fn data_path(&self, base: &std::path::Path) -> PathBuf {
        match self {
            Self::Signatures  => base.join("signatures"),
            Self::YaraRules   => base.join("yara-rules"),
            Self::IocDatabase => base.join("signatures/ioc"),
            Self::AiModel     => base.join("models"),
            Self::Decryptors  => base.join("decryptors"),
            Self::CoreBinaries => PathBuf::from("/opt/aegis/bin"),
            Self::SecurityPolicy => PathBuf::from("/etc/aegis"),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Signatures   => "서명 데이터베이스",
            Self::YaraRules    => "YARA 룰",
            Self::IocDatabase  => "IOC 데이터베이스",
            Self::AiModel      => "AI 모델",
            Self::Decryptors   => "복호화 도구",
            Self::CoreBinaries => "코어 바이너리",
            Self::SecurityPolicy => "보안 정책",
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    match cli.cmd {
        Cmd::Check => {
            channel::check_updates(&cli.data_dir, &cli.channel).await?;
        }
        Cmd::Apply { bundle, only, dry_run } => {
            let targets = if only.is_empty() {
                vec![
                    UpdateTarget::Signatures,
                    UpdateTarget::YaraRules,
                    UpdateTarget::IocDatabase,
                    UpdateTarget::AiModel,
                    UpdateTarget::Decryptors,
                    UpdateTarget::SecurityPolicy,
                ]
            } else {
                only
            };

            if let Some(bundle_path) = bundle {
                bundle::apply_bundle(&bundle_path, &targets, &cli.data_dir, &cli.pub_key, dry_run)?;
            } else {
                channel::apply_online(&targets, &cli.data_dir, &cli.channel, dry_run).await?;
            }
        }
        Cmd::Bundle { output, include } => {
            let targets = if include.is_empty() {
                vec![
                    UpdateTarget::Signatures,
                    UpdateTarget::YaraRules,
                    UpdateTarget::IocDatabase,
                ]
            } else {
                include
            };
            bundle::create_bundle(&output, &targets, &cli.data_dir)?;
        }
        Cmd::Status => {
            manifest::print_status(&cli.data_dir)?;
        }
        Cmd::Rollback { target, version } => {
            manifest::rollback(&target, version.as_deref(), &cli.data_dir)?;
        }
    }

    Ok(())
}
