/// Errors for OS operations. Only `sys-tools` touches the OS raw.
#[derive(Debug, thiserror::Error)]
pub enum SysError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sandbox refused: {0}")]
    SandboxRefused(String),
}
