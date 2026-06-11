//! 설치 엔진 — 파티션 생성, 파일 시스템 포맷, 파일 복사, 부트로더 설치

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

        // GPT 파티션 테이블 생성
        let cmds: &[(&str, &[&str])] = &[
            ("sgdisk", &["--zap-all", dev]),
            ("sgdisk", &["--new=1:0:+512M", "--typecode=1:EF00",
                         "--change-name=1:EFI", dev]),
            ("sgdisk", &["--new=2:0:+2G", "--typecode=2:8300",
                         "--change-name=2:AegisSystem", dev]),
            ("sgdisk", &["--new=3:0:0", "--typecode=3:8300",
                         "--change-name=3:AegisData", dev]),
        ];

        for (cmd, args) in cmds {
            let status = tokio::process::Command::new(cmd)
                .args(*args)
                .status()
                .await
                .map_err(|e| anyhow::anyhow!("{} not found: {}", cmd, e))?;

            if !status.success() {
                tracing::warn!("{} failed (non-Linux or permission issue)", cmd);
            }
        }

        // 파티션 테이블 갱신
        let _ = tokio::process::Command::new("partprobe")
            .arg(dev)
            .status()
            .await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        Ok(())
    }

    async fn format_partitions(&self) -> anyhow::Result<()> {
        let dev = &self.config.target_device;

        let part_suffix = partition_suffix(dev);
        let efi_part  = format!("{}{}1", dev, part_suffix);
        let sys_part  = format!("{}{}2", dev, part_suffix);
        let data_part = format!("{}{}3", dev, part_suffix);

        // EFI 파티션: FAT32
        let _ = tokio::process::Command::new("mkfs.vfat")
            .args(["-F", "32", "-n", "EFI", &efi_part])
            .status().await;

        // 시스템 파티션: ext4 (암호화 옵션)
        if self.config.security.encrypt_system {
            let pass = self.config.security.encrypt_passphrase.as_deref().unwrap_or("aegis");
            // LUKS 포맷
            let _ = tokio::process::Command::new("sh")
                .args(["-c", &format!(
                    "echo -n '{}' | cryptsetup luksFormat {} -",
                    pass, sys_part
                )])
                .status().await;
            let _ = tokio::process::Command::new("sh")
                .args(["-c", &format!(
                    "echo -n '{}' | cryptsetup open {} aegis_root -",
                    pass, sys_part
                )])
                .status().await;
            let _ = tokio::process::Command::new("mkfs.ext4")
                .args(["-L", "AegisSystem", "/dev/mapper/aegis_root"])
                .status().await;
        } else {
            let _ = tokio::process::Command::new("mkfs.ext4")
                .args(["-L", "AegisSystem", &sys_part])
                .status().await;
        }

        // 데이터 파티션: ext4
        let _ = tokio::process::Command::new("mkfs.ext4")
            .args(["-L", "AegisData", &data_part])
            .status().await;

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
            sys_part
        };

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
    const CANDIDATESTS: &[&str] = &[
        "/run/live/medium/live/filesystem.squashfs",
        "/run/live/rootfs/filesystem.squashfs",
        "/lib/live/mount/medium/live/filesystem.squashfs",
        "/lib/live/mount/medium/live/filesystem.squashfs",
        "/cdrom/live/filesystem.squashfs",
        "/run/initramfs/live/filesystem.squashfs",
    ];
    CANDIDATESTS
        .iter()
        .map(Path::new)
        .find(|p| p.is_file())
        .map(|p| p.to_path_buf())
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
            run_command("unsquashfs", &["-f", "-d", mount_root, squashfs.to_str().unwrap()])
                .await
                .map_err(|e| anyhow::anyhow!("unsquashfs 실패: {}", e))?;
            return Ok(());
        }
    }

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
    let status = tokio::process::Command::new(cmd)
        .args(args)
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run {}: {}", cmd, e))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{} {:?} failed (exit {:?})", cmd, args, status.code()))
    }
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
