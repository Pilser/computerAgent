/// Network-namespace args for the bwrap backend.
///
/// `network = true` (default) keeps the host network; `false` adds
/// `--unshare-net`, so the command gets a fresh namespace with only
/// its own private loopback. bwrap backend only.
pub fn network_args(network: bool) -> Vec<String> {
    if network {
        Vec::new()
    } else {
        vec!["--unshare-net".to_string()]
    }
}

/// Whether commands really run without a network: the flag is off and
/// the sandbox is enabled. The backend check lives in `wrap`.
pub fn network_unshared(network: bool, sandbox_enabled: bool) -> bool {
    !network && sandbox_enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_flag_maps_to_unshare_arg() {
        assert!(network_args(true).is_empty());
        assert_eq!(network_args(false), vec!["--unshare-net"]);
        assert!(network_unshared(false, true));
        assert!(!network_unshared(true, true));
        assert!(!network_unshared(false, false));
    }
}
