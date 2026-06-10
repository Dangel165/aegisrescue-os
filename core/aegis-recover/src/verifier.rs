//! 복구 파일 무결성 검증

use std::path::Path;

pub struct VerificationResult {
    pub path: std::path::PathBuf,
    pub is_valid: bool,
    pub reason: String,
}

pub fn verify_recovered_file(path: &Path) -> VerificationResult {
    let ext = path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (is_valid, reason) = match ext.as_str() {
        "jpg" | "jpeg" => verify_jpeg(path),
        "png"          => verify_png(path),
        "pdf"          => verify_pdf(path),
        "zip"          => verify_zip(path),
        _              => (true, "No format-specific validation".into()),
    };

    VerificationResult {
        path: path.to_path_buf(),
        is_valid,
        reason,
    }
}

fn verify_jpeg(path: &Path) -> (bool, String) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => return (false, format!("Read error: {}", e)),
    };

    if data.len() < 4 {
        return (false, "File too small".into());
    }

    let valid_header = data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF;
    let valid_footer = data[data.len()-2] == 0xFF && data[data.len()-1] == 0xD9;

    if valid_header && valid_footer {
        (true, "Valid JPEG (header+footer OK)".into())
    } else {
        (false, format!("Invalid JPEG (header: {}, footer: {})", valid_header, valid_footer))
    }
}

fn verify_png(path: &Path) -> (bool, String) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => return (false, format!("Read error: {}", e)),
    };

    const PNG_HEADER: &[u8] = b"\x89PNG\r\n\x1A\n";
    const PNG_FOOTER: &[u8] = b"IEND\xAEB`\x82";

    let has_header = data.starts_with(PNG_HEADER);
    let has_footer = data.windows(PNG_FOOTER.len()).any(|w| w == PNG_FOOTER);

    if has_header && has_footer {
        (true, "Valid PNG".into())
    } else {
        (false, "Invalid PNG structure".into())
    }
}

fn verify_pdf(path: &Path) -> (bool, String) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => return (false, format!("Read error: {}", e)),
    };

    if data.starts_with(b"%PDF") {
        (true, "Valid PDF header".into())
    } else {
        (false, "Missing PDF header".into())
    }
}

fn verify_zip(path: &Path) -> (bool, String) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => return (false, format!("Read error: {}", e)),
    };

    if data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") {
        (true, "Valid ZIP/Office format".into())
    } else {
        (false, "Invalid ZIP header".into())
    }
}
