/// Build a `zerobox` (macOS) command. Construction only; the
/// caller decides whether to spawn it.
pub fn zerobox_command(
    shell: &str,
    command: &str,
    cwd: &std::path::Path,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("zerobox");
    cmd.arg("--allow-write");
    cmd.arg(cwd.as_os_str());
    cmd.arg("--");
    cmd.arg(shell);
    cmd.arg("-c");
    cmd.arg(command);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerobox_argv_shape() {
        let cwd = std::path::Path::new("/tmp/work");
        let cmd = zerobox_command("sh", "echo hi", cwd);
        let std_cmd = cmd.as_std();
        assert_eq!(std_cmd.get_program().to_string_lossy(), "zerobox");
        let args: Vec<String> = std_cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec!["--allow-write", "/tmp/work", "--", "sh", "-c", "echo hi"]
        );
    }
}
