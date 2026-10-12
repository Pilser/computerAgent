/// Basic host OS facts for prompts and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OsInfo {
    pub os: String,
    pub arch: String,
}

/// Collect the current host's OS facts. Never fails; unknown values
/// fall back to `std::env::consts` compile-time facts.
pub fn gather() -> OsInfo {
    OsInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

impl OsInfo {
    /// One-line human description, e.g. `linux/x86_64`.
    pub fn describe(&self) -> String {
        format!("{}/{}", self.os, self.arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_info_non_empty() {
        let info = gather();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(!info.describe().is_empty());
    }
}
