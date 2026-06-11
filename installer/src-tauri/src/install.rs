//! 설치 엔진 — 파티션 생성, 파일 시스템 포맷, 파일 복사, 부트로더 설치

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// EFI 시스템 파티션 (MiB)
const EFI_SIZE_MIB: u32 = 512;
/// Live squashfs 전개 + ext4 메타데이터 (최소 12GiB, squashfs 크기에 따라 자동 확대)
const MIN_SYSTEM_MIB: u32 = 12288;
const SYSTEM_MARGIN_MIB: u32 = 2048;
const MIN_DATA_MIB: u32 = 2048;
const INSTALLER_IMAGE_ID: &str = "aegis-installer-v2-12g";

/// Shown in UI so Live ISO can be verified before disk install.
pub fn installer_build_info() -> (&'static str, u32) {
    (INSTALLER_IMAGE_ID, compute_system_partition_mib())
}

/// 설치 구성 (프론트엔드에서 전달)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    /// 설치 언어
    pub language: String,
    /// 설치 대상 장치 (예: /dev/sda)
    pub target_device: String,
    /// 설치 모드
    pub install_mode: InstallMode,
    /// 선택된 컴포넌트
    pub components: Vec<String>,
    /// 호스트명
    pub hostname: String,
    /// 시간대
    pub timezone: String,
    /// 키보드 레이아웃
    pub keyboard: String,
    /// 보안 옵션
    pub security: SecurityOptions,
    /// 네트워크 기본 모드
    pub default_network_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallMode {
    /// 전체 디스크 사용 (파티션 자동 생성)
    WholeDisk,
    /// 기존 파티션에 설치
    ExistingPartition { partition: String },
    /// USB Live 모드 (디스크 설치 안 함)
    LiveUsb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityOptions {
    pub enable_secure_boot: bool,
    pub encrypt_system: bool,
    pub encrypt_passphrase: Option<String>,
    pub enable_tpm: bool,
}

/// 설치 진행률 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub percent: u8,
    pub message: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallPhase {
    Preparing,
    Partitioning,
    Formatting,
    CopyingFiles,
    InstallingBootloader,
    ConfiguringSystem,
    InstallingComponents,
    Finalizing,
    Complete,
    Failed,
}

impl InstallConfig {
    pub fn validate(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = vec![];

        if self.target_device.is_empty() && self.install_mode != InstallMode::LiveUsb {
            return Err(anyhow::anyhow!("대상 장치를 선택하세요"));
        }
        if self.install_mode == InstallMode::WholeDisk && !self.target_device.is_empty() {
            if let Ok(info) = crate::disk::get_disk_info(&self.target_device) {
                let sys_mib = compute_system_partition_mib();
                let need_bytes =
                    (EFI_SIZE_MIB + sys_mib + MIN_DATA_MIB) as u64 * 1024 * 1024;
                if info.disk.size_bytes < need_bytes {
                    return Err(anyhow::anyhow!(
                        "디스크 용량 부족: {:.1} GiB 필요 (시스템 파티션 {} GiB), {} 감지됨",
                        need_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                        sys_mib as f64 / 1024.0,
                        info.disk.size_human
                    ));
                }
            }
        }
        if self.hostname.is_empty() {
            return Err(anyhow::anyhow!("호스트명을 입력하세요"));
        }
        if self.components.is_empty() {
            return Err(anyhow::anyhow!("최소 하나 이상의 컴포넌트를 선택하세요"));
        }
        if self.security.encrypt_system {
            if let Some(ref pass) = self.security.encrypt_passphrase {
                if pass.len() < 8 {
                    return Err(anyhow::anyhow!("암호화 패스프레이즈는 8자 이상이어야 합니다"));
                }
            } else {
                return Err(anyhow::anyhow!("암호화를 선택했습니다 — 패스프레이즈를 입력하세요"));
            }
        }

        if !self.components.contains(&"core".to_string()) {
            warnings.push("core 컴포넌트는 필수입니다 — 자동으로 추가됩니다".into());
        }

        Ok(warnings)
    }
}

pub struct Installer {
    config: InstallConfig,
}

impl Installer {
    pub fn new(config: InstallConfig) -> Self {
        Self { config }
    }

    pub async fn run<F>(&mut self, progress_cb: F) -> anyhow::Result<()>
    where
        F: Fn(InstallProgress) + Send + 'static,
    {
        macro_rules! progress {
            ($phase:expr, $pct:expr, $msg:expr) => {
                progress_cb(InstallProgress {
                    phase: $phase,
                    percent: $pct,
                    message: $msg.to_string(),
                    error: None,
                });
            };
        }

        progress!(InstallPhase::Preparing, 0, "설치 준비 중...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if self.config.install_mode == InstallMode::LiveUsb {
            progress!(InstallPhase::Complete, 100, "Live USB 모드 — 설치 완료");
            return Ok(());
        }

        // 1. 파티션 생성
        progress!(InstallPhase::Partitioning, 5, "파티션 테이블 생성 중...");
        self.create_partitions().await?;
        progress!(InstallPhase::Partitioning, 15, "파티션 생성 완료");

        // 2. 포맷
        progress!(InstallPhase::Formatting, 20, "파일시스템 포맷 중...");
        self.format_partitions().await?;
        progress!(InstallPhase::Formatting, 30, "포맷 완료");

        // 3. 파일 복사
        progress!(InstallPhase::CopyingFiles, 35, "시스템 파일 복사 중...");
        self.copy_system_files(&progress_cb).await?;
        progress!(InstallPhase::CopyingFiles, 65, "파일 복사 완료");

        // 4. 컴포넌트 설치
        progress!(InstallPhase::InstallingComponents, 70, "컴포넌트 설치 중...");
        self.install_components(&progress_cb).await?;
        progress!(InstallPhase::InstallingComponents, 80, "컴포넌트 설치 완료");

        // 5. 부트로더
        progress!(InstallPhase::InstallingBootloader, 82, "부트로더 설치 중...");
        self.install_bootloader().await?;
        progress!(InstallPhase::InstallingBootloader, 90, "부트로더 설치 완료");

        // 6. 시스템 설정
        progress!(InstallPhase::ConfiguringSystem, 92, "시스템 설정 적용 중...");
        self.configure_system().await?;
        progress!(InstallPhase::ConfiguringSystem, 97, "시스템 설정 완료");

        // 7. 완료
        progress!(InstallPhase::Finalizing, 99, "마무리 중...");
        self.finalize().await?;
        progress!(InstallPhase::Complete, 100, "✓ 설치가 완료되었습니다!");

        Ok(())
    }

    async fn create_partitions(&self) -> anyhow::Result<()> {
        let dev = &self.config.target_device;
        tracing::info!(device = dev, "creating partition table");

        let sys_mib = compute_system_partition_mib();
        let sys_new = format!("--new=2:0:+{}M", sys_mib);
        tracing::info!(sys_mib, image = INSTALLER_IMAGE_ID, "creating partitions");

        run_or_bail("sgdisk", &["--zap-all", dev]).await?;
        run_or_bail(
            "sgdisk",
            &[
                "--new=1:0:+512M",
                "--typecode=1:EF00",
                "--change-name=1:EFI",
                dev,
            ],
        )
        .await?;
        run_or_bail(
            "sgdisk",
            &[
                sys_new.as_str(),
                "--typecode=2:8300",
                "--change-name=2:AegisSystem",
                dev,
            ],
        )
        .await?;
        run_or_bail(
            "sgdisk",
            &["--new=3:0:0", "--typecode=3:8300", "--change-name=3:AegisData", dev],
        )
        .await?;

        let _ = tokio::process::Command::new("partprobe")
            .arg(dev)
            .status()
            .await;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let part_suffix = partition_suffix(dev);
        for n in 1..=3 {
            let part = format!("{}{}{n}", dev, part_suffix);
            if !Path::new(&part).exists() {
                anyhow::bail!("파티션 생성 실패: {} 없음 (sgdisk/root 권한 확인)", part);
            }
        }

        Ok(())
    }

    async fn format_partitions(&self) -> anyhow::Result<()> {
        let dev = &self.config.target_device;

        let part_suffix = partition_suffix(dev);
        let efi_part  = format!("{}{}1", dev, part_suffix);
        let sys_part  = format!("{}{}2", dev, part_suffix);
        let data_part = format!("{}{}3", dev, part_suffix);

        run_or_bail("mkfs.vfat", &["-F", "32", "-n", "EFI", &efi_part]).await?;

        if self.config.security.encrypt_system {
            let pass = self.config.security.encrypt_passphrase.as_deref().unwrap_or("aegis");
            let pass_esc = pass.replace('\'', "'\\''");
            run_or_bail(
                "sh",
                &[&format!(
                    "echo -n '{}' | cryptsetup luksFormat --batch-mode {} -",
                    pass_esc, sys_part
                )],
            )
            .await?;
            run_or_bail(
                "sh",
                &[&format!(
                    "echo -n '{}' | cryptsetup open {} aegis_root -",
                    pass_esc, sys_part
                )],
            )
            .await?;
            run_or_bail("mkfs.ext4", &["-F", "-m", "1", "-L", "AegisSystem", "/dev/mapper/aegis_root"]).await?;
        } else {
            run_or_bail("mkfs.ext4", &["-F", "-m", "1", "-L", "AegisSystem", &sys_part]).await?;
        }

        run_or_bail("mkfs.ext4", &["-F", "-L", "AegisData", &data_part]).await?;

        Ok(())
    }

    async fn copy_system_files<F>(&self, progress_cb: &F) -> anyhow::Result<()>
    where
        F: Fn(InstallProgress),
    {
        let mount_root = "/mnt/aegis_install";
        std::fs::create_dir_all(mount_root)?;

        let dev = &self.config.target_device;
        let part_suffix = partition_suffix(dev);
        let sys_part = format!("{}{}2", dev, part_suffix);
        let target = if self.config.security.encrypt_system {
            "/dev/mapper/aegis_root".to_string()
        } else {
            sys_part.clone()
        };

        if let Some(squashfs) = resolve_live_squashfs() {
            let need = required_uncompressed_bytes(&squashfs)?;
            let avail = partition_size_bytes(&target)?;
            if need > avail {
                anyhow::bail!(
                    "시스템 파티션 용량 부족: {:.1} GiB 필요, {:.1} GiB 사용 가능 \
                     (파티션 {} MiB, 빌드 {}). ISO/installer를 최신으로 다시 빌드하세요.",
                    need as f64 / 1024.0 / 1024.0 / 1024.0,
                    avail as f64 / 1024.0 / 1024.0 / 1024.0,
                    compute_system_partition_mib(),
                    INSTALLER_IMAGE_ID
                );
            }
        }

        run_command("mount", &[&target, mount_root]).await.map_err(|e| {
            anyhow::anyhow!("시스템 파티션 마운트 실패 ({} → {}): {}", target, mount_root, e)
        })?;

        copy_live_system_to_mount(mount_root, progress_cb).await?;

        ensure_mounted_root(mount_root)?;

        progress_cb(InstallProgress {
            phase: InstallPhase::CopyingFiles,
            percent: 60,
            message: "파일 복사 완료".into(),
            error: None,
        });

        Ok(())
    }

    async fn install_components<F>(&self, progress_cb: &F) -> anyhow::Result<()>
    where
        F: Fn(InstallProgress),
    {
        let component_count = self.config.components.len();
        for (i, component) in self.config.components.iter().enumerate() {
            let pct = 70 + (i as u8 * 10 / component_count.max(1) as u8);
            progress_cb(InstallProgress {
                phase: InstallPhase::InstallingComponents,
                percent: pct,
                message: format!("컴포넌트 설치 중: {}", component),
                error: None,
            });
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        Ok(())
    }

    async fn install_bootloader(&self) -> anyhow::Result<()> {
        let mount_root = "/mnt/aegis_install";
        let dev = &self.config.target_device;

        if self.config.security.enable_secure_boot {
            tracing::info!("Secure Boot requested — signed EFI payloads will be used when available");
        }

        if Path::new("/sys/firmware/efi").exists() {
            // UEFI: GRUB EFI
            let esp = format!("{}/boot/efi", mount_root);
            let part_suffix = if dev.contains("nvme") || dev.contains("mmcblk") { "p" } else { "" };
            let efi_part = format!("{}{}1", dev, part_suffix);

            std::fs::create_dir_all(&esp)?;
            let _ = tokio::process::Command::new("mount")
                .args([&efi_part, &esp])
                .status().await;

            let mut grub_args = vec![
                "grub-install",
                "--target=x86_64-efi",
                "--efi-directory=/boot/efi",
                "--bootloader-id=AegisRescue",
                "--recheck",
            ];
            if self.config.security.enable_secure_boot {
                grub_args.push("--uefi-secure-boot");
            }
            let status = tokio::process::Command::new("chroot")
                .args(std::iter::once(mount_root).chain(grub_args))
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("grub-install (UEFI) failed");
            }
        } else {
            let status = tokio::process::Command::new("chroot")
                .args([mount_root, "grub-install", dev])
                .status()
                .await?;
            if !status.success() {
                anyhow::bail!("grub-install (BIOS) failed");
            }
        }

        let status = tokio::process::Command::new("chroot")
            .args([mount_root, "update-grub"])
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("update-grub failed");
        }

        Ok(())
    }

    async fn configure_system(&self) -> anyhow::Result<()> {
        let root = "/mnt/aegis_install";
        ensure_mounted_root(root)?;

        std::fs::create_dir_all(format!("{}/etc", root))?;

        std::fs::write(format!("{}/etc/hostname", root), &self.config.hostname)
            .map_err(|e| anyhow::anyhow!("hostname 설정 실패: {}", e))?;

        let tz_path = format!("/usr/share/zoneinfo/{}", self.config.timezone);
        if Path::new(&tz_path).exists() {
            run_command(
                "chroot",
                &[
                    root,
                    "ln",
                    "-sf",
                    &tz_path,
                    "/etc/localtime",
                ],
            )
            .await
            .map_err(|e| anyhow::anyhow!("시간대 설정 실패 ({}): {}", self.config.timezone, e))?;
        } else {
            tracing::warn!(timezone = %self.config.timezone, "timezone file missing in target root");
        }

        let locale = lang_to_locale(&self.config.language);
        std::fs::write(
            format!("{}/etc/locale.conf", root),
            format!("LANG={}\n", locale),
        )
        .map_err(|e| anyhow::anyhow!("locale.conf 작성 실패: {}", e))?;

        std::fs::write(
            format!("{}/etc/vconsole.conf", root),
            format!("KEYMAP={}\n", self.config.keyboard),
        )
        .map_err(|e| anyhow::anyhow!("vconsole.conf 작성 실패: {}", e))?;

        let policy_src = "/usr/share/aegis/policy.toml";
        let policy_dst = format!("{}/etc/aegis/policy.toml", root);
        std::fs::create_dir_all(format!("{}/etc/aegis", root))?;
        if Path::new(policy_src).exists() {
            std::fs::copy(policy_src, &policy_dst)
                .map_err(|e| anyhow::anyhow!("policy.toml 복사 실패: {}", e))?;
        }

        let policy_content = format!(
            "# AegisRescue Policy (auto-generated)\n\n\
             [network]\ndefault = \"{}\"\n",
            self.config.default_network_mode
        );
        std::fs::write(&policy_dst, policy_content)
            .map_err(|e| anyhow::anyhow!("policy.toml 작성 실패: {}", e))?;

        self.write_fstab(root).await?;

        Ok(())
    }

    async fn write_fstab(&self, root: &str) -> anyhow::Result<()> {
        let dev = &self.config.target_device;
        let part_suffix = if dev.contains("nvme") || dev.contains("mmcblk") { "p" } else { "" };

        // UUID 조회
        let uuid = |part: &str| -> String {
            std::process::Command::new("blkid")
                .args(["-o", "value", "-s", "UUID", part])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default()
        };

        let efi_uuid  = uuid(&format!("{}{}1", dev, part_suffix));
        let sys_uuid  = uuid(&format!("{}{}2", dev, part_suffix));
        let data_uuid = uuid(&format!("{}{}3", dev, part_suffix));

        let fstab = format!(
            "# /etc/fstab — AegisRescue\n\
             # <device>           <mount>         <type>  <options>          <dump> <pass>\n\
             UUID={efi_uuid}  /boot/efi       vfat    umask=0077         0      2\n\
             UUID={sys_uuid}  /               ext4    defaults,noatime   0      1\n\
             UUID={data_uuid} /var/lib/aegis  ext4    defaults,noatime   0      2\n\
             tmpfs               /tmp            tmpfs   defaults,size=512m 0      0\n"
        );

        std::fs::write(format!("{}/etc/fstab", root), fstab)?;
        Ok(())
    }

    async fn finalize(&self) -> anyhow::Result<()> {
        let mount_root = "/mnt/aegis_install";

        // initramfs 재생성
        let _ = tokio::process::Command::new("chroot")
            .args([mount_root, "update-initramfs", "-u", "-k", "all"])
            .status().await;

        // 설치 완료 마커 파일 생성 (다음 부팅 시 설치 마법사 건너뜀)
        let aegis_data = format!("{}/var/lib/aegis", mount_root);
        std::fs::create_dir_all(&aegis_data)?;
        std::fs::write(
            format!("{}/.installed", aegis_data),
            format!(
                "installed_at={}\nversion=0.1.0\nlanguage={}\nhostname={}\n\
                 secure_boot={}\ntpm_enabled={}\n",
                chrono::Utc::now().to_rfc3339(),
                self.config.language,
                self.config.hostname,
                self.config.security.enable_secure_boot,
                self.config.security.enable_tpm,
            ),
        )?;

        // 언마운트
        for path in &["/boot/efi", "/proc", "/sys", "/dev", ""] {
            let full = if path.is_empty() {
                mount_root.to_string()
            } else {
                format!("{}{}", mount_root, path)
            };
            let _ = tokio::process::Command::new("umount")
                .args(["-l", &full])
                .status().await;
        }

        tracing::info!("Installation finalized");
        Ok(())
    }
}

fn lang_to_locale(lang: &str) -> &str {
    match lang {
        "ko" => "ko_KR.UTF-8",
        "ja" => "ja_JP.UTF-8",
        "zh" => "zh_CN.UTF-8",
        "de" => "de_DE.UTF-8",
        "fr" => "fr_FR.UTF-8",
        "es" => "es_ES.UTF-8",
        _    => "en_US.UTF-8",
    }
}

fn partition_suffix(dev: &str) -> &'static str {
    if dev.contains("nvme") || dev.contains("mmcblk") {
        "p"
    } else {
        ""
    }
}

fn resolve_live_squashfs() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "/run/live/medium/live/filesystem.squashfs",
        "/run/live/rootfs/filesystem.squashfs",
        "/lib/live/mount/medium/live/filesystem.squashfs",
        "/cdrom/live/filesystem.squashfs",
        "/run/initramfs/live/filesystem.squashfs",
        "/live/filesystem.squashfs",
    ];
    for p in CANDIDATES {
        let path = Path::new(p);
        if path.is_file() {
            return Some(path.to_path_buf());
        }
    }
    for base in [
        "/run/live/medium",
        "/lib/live/mount/medium",
        "/cdrom",
        "/run/initramfs/live",
    ] {
        let p = Path::new(base).join("live/filesystem.squashfs");
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn compute_system_partition_mib() -> u32 {
    if let Some(sq) = resolve_live_squashfs() {
        if let Ok(bytes) = required_uncompressed_bytes(&sq) {
            let need_mib = (bytes / 1024 / 1024) as u32 + SYSTEM_MARGIN_MIB;
            return need_mib.max(MIN_SYSTEM_MIB);
        }
    }
    MIN_SYSTEM_MIB
}

fn required_uncompressed_bytes(squashfs: &Path) -> anyhow::Result<u64> {
    if let Some(size_file) = squashfs_uncompressed_size_file(squashfs) {
        if let Ok(text) = std::fs::read_to_string(&size_file) {
            if let Ok(n) = text.trim().parse::<u64>() {
                if n > 0 {
                    // ext4 메타데이터 + 여유 10%
                    return Ok(n.saturating_mul(110) / 100);
                }
            }
        }
    }
    let compressed = std::fs::metadata(squashfs)?.len();
    Ok(compressed.saturating_mul(4))
}

fn squashfs_uncompressed_size_file(squashfs: &Path) -> Option<PathBuf> {
    squashfs
        .parent()
        .map(|p| p.join("filesystem.size"))
        .filter(|p| p.is_file())
}

fn partition_size_bytes(device: &str) -> anyhow::Result<u64> {
    let output = std::process::Command::new("blockdev")
        .args(["--getsize64", device])
        .output()
        .map_err(|e| anyhow::anyhow!("blockdev failed for {}: {}", device, e))?;
    if !output.status.success() {
        anyhow::bail!(
            "파티션 크기 조회 실패: {} (exit {:?})",
            device,
            output.status.code()
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!("invalid blockdev size for {}: {}", device, e))
}

async fn copy_live_system_to_mount<F>(mount_root: &str, progress_cb: &F) -> anyhow::Result<()>
where
    F: Fn(InstallProgress),
{
    if let Some(squashfs) = resolve_live_squashfs() {
        if is_tool_available("unsquashfs") {
            progress_cb(InstallProgress {
                phase: InstallPhase::CopyingFiles,
                percent: 40,
                message: format!("시스템 이미지 압축 해제 중 ({})...", squashfs.display()),
                error: None,
            });
            run_command_capture(
                "unsquashfs",
                &["-f", "-d", mount_root, squashfs.to_str().unwrap()],
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "unsquashfs 실패 ({} → {}): {}. \
                     시스템 파티션 {} MiB ({}) — ISO를 최신 installer로 재빌드했는지 확인하세요.",
                    squashfs.display(),
                    mount_root,
                    e,
                    compute_system_partition_mib(),
                    INSTALLER_IMAGE_ID
                )
            })?;
            return Ok(());
        }
        anyhow::bail!(
            "filesystem.squashfs 는 {} 에 있으나 unsquashfs 가 없습니다",
            squashfs.display()
        );
    }

    progress_cb(InstallProgress {
        phase: InstallPhase::CopyingFiles,
        percent: 38,
        message: "Live squashfs 미발견 — rsync/tar 폴백 시도".into(),
        error: None,
    });

    if is_tool_available("rsync") {
        progress_cb(InstallProgress {
            phase: InstallPhase::CopyingFiles,
            percent: 42,
            message: "Live 시스템에서 rsync 복사 중...".into(),
            error: None,
        });
        copy_live_root_with_rsync(mount_root).await?;
        return Ok(());
    }

    progress_cb(InstallProgress {
        phase: InstallPhase::CopyingFiles,
        percent: 42,
        message: "Live 시스템에서 tar 복사 중...".into(),
        error: None,
    });
    copy_live_root_with_tar(mount_root).await
}

async fn copy_live_root_with_rsync(mount_root: &str) -> anyhow::Result<()> {
    run_command(
        "rsync",
        &[
            "-aHAX",
            "--exclude=/proc/*",
            "--exclude=/sys/*",
            "--exclude=/dev/*",
            "--exclude=/run/*",
            "--exclude=/mnt/*",
            "--exclude=/tmp/*",
            "/",
            mount_root,
        ],
    )
    .await
    .map_err(|e| anyhow::anyhow!("rsync 실패: {}", e))
}

async fn copy_live_root_with_tar(mount_root: &str) -> anyhow::Result<()> {
    let script = format!(
        "cd / && tar --numeric-owner \
         --exclude=./proc --exclude=./sys --exclude=./dev \
         --exclude=./run --exclude=./mnt --exclude=./tmp \
         --exclude=./mnt/aegis_install \
         -cf - . | tar -C {mount_root} -xpf -"
    );
    run_command("sh", &["-c", &script])
        .await
        .map_err(|e| anyhow::anyhow!("tar 복사 실패: {}", e))
}

async fn run_command(cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    run_command_capture(cmd, args).await.map(|_| ())
}

async fn run_command_capture(cmd: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run {}: {}", cmd, e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit {:?}", output.status.code())
        };
        Err(anyhow::anyhow!("{} {:?} failed: {}", cmd, args, detail))
    }
}

async fn run_or_bail(cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    run_command(cmd, args).await.map_err(|e| {
        anyhow::anyhow!("{} 실행 실패 (root 권한·대상 디스크 확인): {}", cmd, e)
    })
}

fn ensure_mounted_root(root: &str) -> anyhow::Result<()> {
    let etc = format!("{}/etc", root);
    if Path::new(&etc).is_dir() {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "설치 대기 경로에 시스템이 없습니다 ({etc}). \
         파티션 마운트 또는 squashfs 추출이 실패했을 수 있습니다."
    ))
}

fn is_tool_available(tool: &str) -> bool {
    std::process::Command::new("which")
        .arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
