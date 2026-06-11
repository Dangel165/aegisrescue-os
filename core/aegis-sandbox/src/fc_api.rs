//! Firecracker REST API (HTTP/1.1 over Unix domain socket)

#[cfg(unix)]
mod imp {
    use anyhow::{bail, Context};
    use std::path::{Path, PathBuf};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;
    use tokio::process::{Child, Command};
    use tokio::time::{sleep, Duration};

    pub struct FirecrackerSession {
        child: Child,
        socket_path: PathBuf,
    }

    impl FirecrackerSession {
        pub async fn start() -> anyhow::Result<Self> {
            let fc_bin = find_firecracker()?;
            let socket_path = PathBuf::from(format!("/tmp/aegis_fc_{}.sock", std::process::id()));
            let _ = std::fs::remove_file(&socket_path);

            let mut child = Command::new(&fc_bin)
                .arg("--api-sock")
                .arg(&socket_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .with_context(|| format!("failed to spawn {}", fc_bin.display()))?;

            for _ in 0..50 {
                if socket_path.exists() {
                    return Ok(Self { child, socket_path });
                }
                sleep(Duration::from_millis(50)).await;
            }

            let _ = child.kill().await;
            bail!("firecracker API socket did not appear: {}", socket_path.display());
        }

        pub async fn configure(&self, sample: &Path) -> anyhow::Result<()> {
            let kernel = Path::new(super::VM_DIR).join("vmlinux.bin");
            let rootfs = Path::new(super::VM_DIR).join("rootfs.ext4");

            if !kernel.exists() || !rootfs.exists() {
                bail!(
                    "VM artifacts missing under {} (run tools/build-vm-artifacts.sh)",
                    super::VM_DIR
                );
            }

            let boot = serde_json::json!({
                "kernel_image_path": kernel,
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off init=/sbin/init"
            });
            put_json(&self.socket_path, "/boot-source", &boot).await?;

            put_json(
                &self.socket_path,
                "/drives/rootfs",
                &serde_json::json!({
                    "drive_id": "rootfs",
                    "path_on_host": rootfs,
                    "is_root_device": true,
                    "is_read_only": false
                }),
            )
            .await?;

            put_json(
                &self.socket_path,
                "/drives/sample",
                &serde_json::json!({
                    "drive_id": "sample",
                    "path_on_host": sample,
                    "is_root_device": false,
                    "is_read_only": true
                }),
            )
            .await?;

            put_json(
                &self.socket_path,
                "/machine-config",
                &serde_json::json!({
                    "vcpu_count": 1,
                    "mem_size_mib": 256,
                    "smt": false,
                    "track_dirty_pages": false
                }),
            )
            .await?;

            Ok(())
        }

        pub async fn start_instance(&self) -> anyhow::Result<()> {
            put_json(
                &self.socket_path,
                "/actions",
                &serde_json::json!({ "action_type": "InstanceStart" }),
            )
            .await
        }

        pub async fn shutdown(&mut self) -> anyhow::Result<()> {
            let _ = put_json(
                &self.socket_path,
                "/actions",
                &serde_json::json!({ "action_type": "SendCtrlAltDel" }),
            )
            .await;
            sleep(Duration::from_millis(200)).await;
            let _ = self.child.kill().await;
            let _ = std::fs::remove_file(&self.socket_path);
            Ok(())
        }
    }

    impl Drop for FirecrackerSession {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }

    async fn put_json(socket: &Path, path: &str, body: &serde_json::Value) -> anyhow::Result<()> {
        let payload = serde_json::to_string(body)?;
        let request = format!(
            "PUT {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );

        let mut stream = UnixStream::connect(socket)
            .await
            .with_context(|| format!("connect firecracker socket {}", socket.display()))?;
        stream.write_all(request.as_bytes()).await?;

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let response = String::from_utf8_lossy(&buf[..n]);
        if !response.contains("HTTP/1.1 204") && !response.contains("HTTP/1.1 200") {
            bail!("firecracker PUT {path} failed: {response}");
        }
        Ok(())
    }

    fn find_firecracker() -> anyhow::Result<PathBuf> {
        for candidate in ["/usr/bin/firecracker", "/usr/local/bin/firecracker"] {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return Ok(p);
            }
        }
        anyhow::bail!("firecracker binary not found")
    }

    pub fn firecracker_available() -> bool {
        find_firecracker().is_ok()
    }
}

#[cfg(not(unix))]
mod imp {
    use std::path::Path;

    pub struct FirecrackerSession;

    impl FirecrackerSession {
        pub async fn start() -> anyhow::Result<Self> {
            anyhow::bail!("Firecracker is only supported on Linux")
        }

        pub async fn configure(&self, _sample: &Path) -> anyhow::Result<()> {
            Ok(())
        }

        pub async fn start_instance(&self) -> anyhow::Result<()> {
            Ok(())
        }

        pub async fn shutdown(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    pub fn firecracker_available() -> bool {
        false
    }
}

#[cfg(unix)]
pub(crate) const VM_DIR: &str = "/usr/share/aegis/vm";

pub use imp::firecracker_available;
pub use imp::FirecrackerSession;
