use crate::error::SysError;
use crate::sandbox::types::Sandbox;

impl Sandbox {
    fn bare_command(&self, command: &str) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(self.shell());
        cmd.arg("-c").arg(command);
        cmd.kill_on_drop(true);
        cmd
    }

    /// Wrap a shell command in the sandbox backend.
    ///
    /// Disabled sandboxes pass through bare. A required sandbox whose
    /// backend is missing is refused; an optional one falls back to
    /// bare. Construction only; use [`Sandbox::output_command`] to run.
    pub fn wrap_command(&self, command: &str) -> Result<tokio::process::Command, SysError> {
        if !self.is_enabled() {
            return Ok(self.bare_command(command));
        }
        if let Some(reason) = self.refusal_reason() {
            return Err(SysError::SandboxRefused(reason));
        }
        if !self.backend_available() {
            return Ok(self.bare_command(command));
        }
        if self.backend() == "zerobox" {
            let cwd = std::env::current_dir().unwrap_or_default();
            return Ok(super::zerobox::zerobox_command(self.shell(), command, &cwd));
        }
        let masks: Vec<std::path::PathBuf> = super::masks::builtin_mask_roots()
            .into_iter()
            .filter(|root| root.exists())
            .collect();
        Ok(super::bwrap::bwrap_command(
            self.shell(),
            command,
            &masks,
            self.network(),
        ))
    }

    /// Wrap and run a shell command, capturing its output.
    pub async fn output_command(&self, command: &str) -> Result<std::process::Output, SysError> {
        Ok(self.wrap_command(command)?.output().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(cmd: &tokio::process::Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn wrap_bare_passthrough_when_disabled() {
        let sb = Sandbox::new();
        let cmd = sb.wrap_command("echo hi").unwrap();
        assert_eq!(cmd.as_std().get_program().to_string_lossy(), "sh");
        assert_eq!(argv(&cmd), vec!["-c", "echo hi"]);
    }

    #[test]
    fn required_missing_backend_refuses() {
        let sb = Sandbox::new()
            .with_enabled(true)
            .with_required(true)
            .with_backend("ca-definitely-missing-backend-xyz");
        let err = sb.wrap_command("echo hi").unwrap_err();
        assert!(matches!(err, SysError::SandboxRefused(_)));
    }

    #[test]
    fn optional_missing_backend_falls_back_to_bare() {
        let sb = Sandbox::new()
            .with_enabled(true)
            .with_backend("ca-definitely-missing-backend-xyz");
        let cmd = sb.wrap_command("echo hi").unwrap();
        assert_eq!(argv(&cmd), vec!["-c", "echo hi"]);
    }
}
