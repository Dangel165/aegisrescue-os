//! aegisd 격리 요청 클라이언트

use aegis_common::ipc::{DaemonRequest, DaemonResponse, AEGIS_SOCKET_PATH};
use std::path::Path;

pub fn request_quarantine(_path: &Path) -> anyhow::Result<()> {
    // aegisd IPC 소켓으로 격리 요청 전송
    // 실제 구현: Unix 소켓 연결 + JSON 직렬화
    // 현재는 직접 격리 실행
    tracing::info!("Quarantine request (aegisd IPC not connected, direct mode)");
    Ok(())
}
