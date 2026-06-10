//! aegisd IPC 클라이언트

use aegis_common::ipc::{DaemonRequest, DaemonResponse, AEGIS_SOCKET_PATH};

pub async fn send_request(req: &DaemonRequest) -> anyhow::Result<DaemonResponse> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let stream = UnixStream::connect(AEGIS_SOCKET_PATH).await
        .map_err(|_| anyhow::anyhow!(
            "Cannot connect to aegisd. Is it running?\n  Run: aegisd --foreground"
        ))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut msg = serde_json::to_string(req)?;
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await?;

    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: DaemonResponse = serde_json::from_str(response_line.trim())?;
    Ok(response)
}

pub fn send_request_sync(req: &DaemonRequest) -> anyhow::Result<DaemonResponse> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(send_request(req))
}
