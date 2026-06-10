//! 마운트 관리 핵심 로직

use aegis_common::error::{AegisError, Result};
use aegis_common::hash::ForensicHashes;
use aegis_common::types::{FilesystemType, MountInfo, MountOptions};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::fstype::detect_filesystem;

pub struct MountManager {
    /// 관리 중인 마운트 목록 (mount_point → MountInfo)
    mounts: Mutex<HashMap<PathBuf, MountInfo>>,
    base_mount_dir: PathBuf,
}

impl MountManager {
    pub fn new() -> Self {
        Self {
            mounts: Mutex::new(HashMap::new()),
            base_mount_dir: PathBuf::from("/mnt/targets"),
        }
    }

    pub async fn mount(
        &self,
        device: &Path,
        target: Option<&Path>,
        opts: MountOptions,
    ) -> Result<MountInfo> {
        // 파일시스템 감지
        let fs_type = detect_filesystem(device)
            .unwrap_or(FilesystemType::Unknown);

        if fs_type == FilesystemType::Unknown {
            warn!("Unknown filesystem on {}", device.display());
        }

        // 마운트 포인트 결정
        let mount_id = Uuid::new_v4();
        let mount_point = target
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.base_mount_dir.join(mount_id.to_string()));

        std::fs::create_dir_all(&mount_point)?;

        // 마운트 전 해시 (파티션 부트섹터)
        let pre_hash = ForensicHashes::compute(device)
            .ok()
            .map(|h| h.sha256);

        // 마운트 옵션 문자열 구성
        let mount_opts = build_mount_options(&opts, &fs_type);

        // 실제 마운트 실행
        self.do_mount(device, &mount_point, &fs_type, &mount_opts)?;

        let info = MountInfo {
            id: mount_id,
            source: device.to_path_buf(),
            target: mount_point.clone(),
            filesystem: fs_type,
            options: opts,
            mounted_at: Utc::now(),
            operator: whoami(),
            reason: None,
            pre_mount_hash: pre_hash,
        };

        info!(
            id = %mount_id,
            source = %device.display(),
            target = %mount_point.display(),
            read_only = info.options.read_only,
            "disk mounted"
        );

        self.mounts.lock().unwrap().insert(mount_point, info.clone());
        Ok(info)
    }

    pub async fn umount(&self, target: &Path) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            nix::mount::umount(target)
                .map_err(|e| AegisError::Mount(format!("umount failed: {}", e)))?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            let status = std::process::Command::new("umount")
                .arg(target)
                .status()
                .map_err(|e| AegisError::Mount(e.to_string()))?;
            if !status.success() {
                return Err(AegisError::Mount("umount failed".into()));
            }
        }

        self.mounts.lock().unwrap().remove(target);
        info!(target = %target.display(), "disk unmounted");
        Ok(())
    }

    pub fn list_mounts(&self) -> Result<Vec<MountInfo>> {
        Ok(self.mounts.lock().unwrap().values().cloned().collect())
    }

    fn do_mount(
        &self,
        device: &Path,
        mount_point: &Path,
        fs_type: &FilesystemType,
        options: &str,
    ) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use nix::mount::{mount, MsFlags};
            let fs_str = fs_type_to_str(fs_type);
            let mut flags = MsFlags::MS_NOEXEC | MsFlags::MS_NODEV | MsFlags::MS_NOSUID;
            if options.contains("ro") {
                flags |= MsFlags::MS_RDONLY;
            }

            mount(
                Some(device),
                mount_point,
                Some(fs_str.as_str()),
                flags,
                Some(options),
            ).map_err(|e| AegisError::Mount(format!("mount({}) failed: {}", device.display(), e)))?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            // 비-Linux 환경(테스트용): mount 커맨드 호출
            let args = vec![
                "-t".to_string(),
                fs_type_to_str(fs_type),
                "-o".to_string(),
                options.to_string(),
                device.to_string_lossy().into_owned(),
                mount_point.to_string_lossy().into_owned(),
            ];

            let status = std::process::Command::new("mount")
                .args(&args)
                .status()
                .map_err(|e| AegisError::Mount(e.to_string()))?;

            if !status.success() {
                return Err(AegisError::Mount(format!(
                    "mount command failed for {}",
                    device.display()
                )));
            }
        }

        Ok(())
    }
}

fn build_mount_options(opts: &MountOptions, fs: &FilesystemType) -> String {
    let mut parts = vec![];

    if opts.read_only { parts.push("ro"); }
    if opts.noexec    { parts.push("noexec"); }
    if opts.nodev     { parts.push("nodev"); }
    if opts.nosuid    { parts.push("nosuid"); }

    // NTFS 전용
    if matches!(fs, FilesystemType::Ntfs) {
        parts.push("windows_names");
        parts.push("norecover");
    }

    parts.join(",")
}

fn fs_type_to_str(fs: &FilesystemType) -> String {
    match fs {
        FilesystemType::Ntfs    => "ntfs3".into(),
        FilesystemType::ExFat   => "exfat".into(),
        FilesystemType::Fat32 | FilesystemType::Fat16 => "vfat".into(),
        FilesystemType::Ext4    => "ext4".into(),
        FilesystemType::Ext3    => "ext3".into(),
        FilesystemType::Ext2    => "ext2".into(),
        FilesystemType::Btrfs   => "btrfs".into(),
        FilesystemType::Xfs     => "xfs".into(),
        FilesystemType::F2fs    => "f2fs".into(),
        FilesystemType::HfsPlus => "hfsplus".into(),
        FilesystemType::Apfs    => "apfs".into(), // apfs-fuse
        _                       => "auto".into(),
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "aegis".into())
}
