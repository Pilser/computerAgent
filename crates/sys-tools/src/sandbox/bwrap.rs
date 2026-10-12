use std::path::PathBuf;

/// Build a `bwrap` (Linux) command. Construction only; the caller
/// decides whether to spawn it.
pub fn bwrap_command(
    shell: &str,
    command: &str,
    masks: &[PathBuf],
    network: bool,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("bwrap");
    cmd.arg("--clearenv");
    cmd.args(["--ro-bind", "/", "/"]);
    for root in masks {
        cmd.arg("--tmpfs");
        cmd.arg(root.as_os_str());
    }
    cmd.args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]);
    cmd.args([
        "--unshare-ipc",
        "--unshare-pid",
        "--unshare-uts",
        "--unshare-cgroup",
    ]);
    for arg in super::network::network_args(network) {
        cmd.arg(arg);
    }
    cmd.args(["--die-with-parent", shell, "-c", command]);
    cmd
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
    fn bwrap_argv_masks_and_unshare_net() {
        let masks = vec![PathBuf::from("/home/u/.ssh")];
        let args = argv(&bwrap_command("sh", "echo hi", &masks, false));
        let tmpfs_at = args.iter().position(|a| a == "--tmpfs").unwrap();
        assert_eq!(args[tmpfs_at + 1], "/home/u/.ssh");
        assert!(args.contains(&"--unshare-net".to_string()));
        assert!(args.contains(&"echo hi".to_string()));
    }

    #[test]
    fn bwrap_argv_keeps_network_by_default() {
        let args = argv(&bwrap_command("sh", "echo hi", &[], true));
        assert!(!args.contains(&"--unshare-net".to_string()));
    }
}
