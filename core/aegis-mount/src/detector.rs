//! 마운트 전 감염 지표 사전 탐지

use std::path::Path;

#[derive(Debug)]
pub struct InfectionIndicator {
    pub severity: &'static str,
    pub description: String,
}

pub async fn detect_infection_indicators(device: &Path) -> Vec<InfectionIndicator> {
    let mut indicators = vec![];

    // MBR 변조 확인 (첫 512바이트 분석)
    if let Ok(mut f) = std::fs::File::open(device) {
        use std::io::Read;
        let mut mbr = [0u8; 512];
        if f.read_exact(&mut mbr).is_ok() {
            // 부트 시그니처 확인
            if mbr[510] != 0x55 || mbr[511] != 0xAA {
                indicators.push(InfectionIndicator {
                    severity: "HIGH",
                    description: "MBR boot signature missing or corrupted (possible bootkit)".into(),
                });
            }

            // 알려진 부트킷 시그니처 확인
            let known_bootkit_bytes: &[&[u8]] = &[
                b"\xEB\x58\x90\x00\x00",  // TDL4 시그니처 패턴 예시
            ];
            for sig in known_bootkit_bytes {
                if mbr.windows(sig.len()).any(|w| w == *sig) {
                    indicators.push(InfectionIndicator {
                        severity: "CRITICAL",
                        description: format!("Known bootkit signature detected in MBR"),
                    });
                }
            }
        }
    }

    // 엔트로피 확인 (높은 엔트로피 = 암호화/랜섬웨어 흔적)
    if let Ok(entropy) = aegis_common::hash::file_entropy(device) {
        if entropy > 7.8 {
            indicators.push(InfectionIndicator {
                severity: "MEDIUM",
                description: format!(
                    "Very high disk entropy ({:.2} bits/byte) — possible encryption by ransomware",
                    entropy
                ),
            });
        }
    }

    indicators
}
