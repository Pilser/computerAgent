/// Sandbox configuration. Split port of vendor `sandbox.rs`.
///
/// DEFAULT OFF: [`Sandbox::new()`] has `enabled = false`, so commands
/// run bare (full-privilege assist mode). Opt in with `with_enabled(true)`.
#[derive(Debug, Clone)]
pub struct Sandbox {
    enabled: bool,
    required: bool,
    backend: String,
    shell: String,
    network: bool,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox {
    /// Default sandbox: disabled, optional, `bwrap` backend, network on.
    pub fn new() -> Self {
        Self {
            enabled: false,
            required: false,
            backend: "bwrap".to_string(),
            shell: "sh".to_string(),
            network: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// When required, commands are refused instead of running bare
    /// if the backend binary is missing.
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_backend(mut self, backend: &str) -> Self {
        if !backend.is_empty() {
            self.backend = backend.to_string();
        }
        self
    }

    pub fn with_shell(mut self, shell: &str) -> Self {
        if !shell.is_empty() {
            self.shell = shell.to_string();
        }
        self
    }

    /// When false, sandboxed commands run without network (bwrap only).
    pub fn with_network(mut self, network: bool) -> Self {
        self.network = network;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_required(&self) -> bool {
        self.required
    }

    pub fn backend(&self) -> &str {
        &self.backend
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn network(&self) -> bool {
        self.network
    }

    /// Binary this sandbox probes for. Known backends are `bwrap`
    /// (default) and `zerobox`; any other name is probed literally so
    /// a missing custom backend falls back to bare (or is refused
    /// when required) instead of silently using bwrap's probe.
    pub(crate) fn backend_binary(&self) -> &str {
        self.backend.as_str()
    }

    /// PATH probe for the backend binary (no shell-out to `which`).
    pub(crate) fn backend_available(&self) -> bool {
        let name = self.backend_binary();
        std::env::var_os("PATH")
            .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
    }

    /// `Some(reason)` when the sandbox is required but its backend is
    /// missing, meaning commands must be refused, not run bare.
    pub fn refusal_reason(&self) -> Option<String> {
        if !self.enabled || !self.required {
            return None;
        }
        if self.backend_available() {
            return None;
        }
        Some(format!(
            "sandbox backend '{}' not found; refusing to run because sandbox-required is set",
            self.backend
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_default_off() {
        let sb = Sandbox::new();
        assert!(!sb.is_enabled());
        assert!(!sb.is_required());
        assert!(sb.network());
        assert_eq!(Sandbox::default().is_enabled(), false);
    }

    #[test]
    fn builder_sets_fields() {
        let sb = Sandbox::new()
            .with_enabled(true)
            .with_required(true)
            .with_backend("zerobox")
            .with_network(false);
        assert!(sb.is_enabled());
        assert!(sb.is_required());
        assert_eq!(sb.backend(), "zerobox");
        assert!(!sb.network());
    }
}
