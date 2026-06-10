//! seccomp-bpf 필터 프로파일 정의

/// 분석 프로세스에 적용할 허용 시스템 콜 목록
/// (이 외 모든 syscall은 SIGKILL)
pub const ALLOWED_SYSCALLS: &[&str] = &[
    "read", "write", "open", "openat", "close",
    "stat", "fstat", "lstat", "poll", "lseek",
    "mmap", "mprotect", "munmap", "brk",
    "rt_sigaction", "rt_sigprocmask",
    "access", "pipe", "select", "sched_yield",
    "dup", "dup2", "nanosleep",
    "getpid", "getuid", "getgid",
    "exit", "exit_group",
    "getcwd", "chdir",
    "readlink", "readlinkat",
    "getdents64", "getdents",
    "uname",
];

/// seccomp 프로파일 JSON (libseccomp 형식)
pub fn seccomp_profile() -> serde_json::Value {
    serde_json::json!({
        "defaultAction": "SCMP_ACT_KILL",
        "syscalls": ALLOWED_SYSCALLS.iter().map(|s| serde_json::json!({
            "name": s,
            "action": "SCMP_ACT_ALLOW"
        })).collect::<Vec<_>>()
    })
}
