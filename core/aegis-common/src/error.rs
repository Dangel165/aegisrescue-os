use thiserror::Error;

pub type Result<T> = std::result::Result<T, AegisError>;

#[derive(Debug, Error)]
pub enum AegisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Mount error: {0}")]
    Mount(String),

    #[error("Filesystem unsupported: {0}")]
    UnsupportedFilesystem(String),

    #[error("Scan error: {0}")]
    Scan(String),

    #[error("Signature engine error: {0}")]
    SignatureEngine(String),

    #[error("YARA error: {0}")]
    Yara(String),

    #[error("AI inference error: {0}")]
    AiInference(String),

    #[error("Recovery error: {0}")]
    Recovery(String),

    #[error("Forensic error: {0}")]
    Forensic(String),

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Quarantine error: {0}")]
    Quarantine(String),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("Boot repair error: {0}")]
    BootRepair(String),

    #[error("Image error: {0}")]
    Image(String),

    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AegisError {
    pub fn is_recoverable(&self) -> bool {
        !matches!(self, AegisError::PermissionDenied(_) | AegisError::PolicyViolation(_))
    }

    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AegisError::PermissionDenied(_) | AegisError::PolicyViolation(_) => ErrorSeverity::Fatal,
            AegisError::HashMismatch { .. } | AegisError::Mount(_) => ErrorSeverity::Error,
            AegisError::NotFound(_) | AegisError::Cancelled => ErrorSeverity::Warning,
            _ => ErrorSeverity::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Fatal,
    Error,
    Warning,
    Info,
}
