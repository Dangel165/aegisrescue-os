//! 파일시스템 타입 자동 감지 (매직 바이트 기반)

use aegis_common::types::FilesystemType;
use std::io::Read;
use std::path::Path;

/// 블록 장치 또는 이미지 파일의 파일시스템 타입 감지
pub fn detect_filesystem(device: &Path) -> Option<FilesystemType> {
    let mut f = std::fs::File::open(device).ok()?;
    let mut buf = [0u8; 4096];
    f.read(&mut buf).ok()?;

    // NTFS: offset 3, "NTFS    "
    if &buf[3..7] == b"NTFS" {
        return Some(FilesystemType::Ntfs);
    }

    // FAT/exFAT: Boot sector signature 0x55AA at offset 510
    if buf[510] == 0x55 && buf[511] == 0xAA {
        // exFAT: "EXFAT   " at offset 3
        if &buf[3..8] == b"EXFAT" {
            return Some(FilesystemType::ExFat);
        }
        // FAT32: "FAT32   " at offset 82
        if &buf[82..87] == b"FAT32" {
            return Some(FilesystemType::Fat32);
        }
        // FAT16: "FAT16   " at offset 54
        if &buf[54..59] == b"FAT16" {
            return Some(FilesystemType::Fat16);
        }
    }

    // ext2/3/4: magic 0xEF53 at offset 0x438
    if buf.len() > 0x43A && buf[0x438] == 0x53 && buf[0x439] == 0xEF {
        // rev level at 0x43C: 1 = ext4
        let feature_incompat = u32::from_le_bytes([
            buf[0x460], buf[0x461], buf[0x462], buf[0x463]
        ]);
        return if feature_incompat & 0x40 != 0 {
            Some(FilesystemType::Ext4)
        } else {
            Some(FilesystemType::Ext3)
        };
    }

    // Btrfs: "_BHRfS_M" at offset 0x10040
    // (requires larger read — fallback to blkid)

    // HFS+: "BD" at offset 1024
    if buf.len() > 1026 && buf[1024] == 0x00 && buf[1025] == 0x04 {
        return Some(FilesystemType::HfsPlus);
    }

    // APFS: "NXSB" at offset 32
    if buf.len() > 36 && &buf[32..36] == b"NXSB" {
        return Some(FilesystemType::Apfs);
    }

    // XFS: "XFSB" at offset 0
    if &buf[0..4] == b"XFSB" {
        return Some(FilesystemType::Xfs);
    }

    // Btrfs: magic at offset 65600 (beyond 4KB, use blkid)
    // Fallback: 시스템의 blkid 사용
    detect_via_blkid(device)
}

fn detect_via_blkid(device: &Path) -> Option<FilesystemType> {
    let output = std::process::Command::new("blkid")
        .args(["-o", "value", "-s", "TYPE", &device.to_string_lossy()])
        .output()
        .ok()?;

    let fs_str = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();

    Some(match fs_str.as_str() {
        "ntfs"    => FilesystemType::Ntfs,
        "exfat"   => FilesystemType::ExFat,
        "vfat"    => FilesystemType::Fat32,
        "ext4"    => FilesystemType::Ext4,
        "ext3"    => FilesystemType::Ext3,
        "ext2"    => FilesystemType::Ext2,
        "btrfs"   => FilesystemType::Btrfs,
        "xfs"     => FilesystemType::Xfs,
        "f2fs"    => FilesystemType::F2fs,
        "hfsplus" => FilesystemType::HfsPlus,
        "apfs"    => FilesystemType::Apfs,
        "swap"    => FilesystemType::Swap,
        _         => FilesystemType::Unknown,
    })
}
