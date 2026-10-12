use std::path::{Path, PathBuf};

/// Well-known credential stores directly under the home directory.
/// Sandboxed commands see these as empty, never as the user's keys.
pub const HOME_MASK_DIRS: [&str; 5] = [".ssh", ".aws", ".gnupg", ".kube", ".docker"];

/// The same, relative to the XDG config base.
pub const CONFIG_MASK_DIRS: [&str; 4] = ["gh", "gcloud", "op", "sops/age"];

/// Built-in mask list for a home dir and XDG config base. Pure, so it
/// can be asserted on without touching the process environment.
pub fn mask_roots_for(home: &Path, xdg_config: Option<&Path>) -> Vec<PathBuf> {
    let config_base = match xdg_config {
        Some(dir) if dir.is_absolute() => dir.to_path_buf(),
        _ => home.join(".config"),
    };
    HOME_MASK_DIRS
        .iter()
        .map(|rel| home.join(rel))
        .chain(CONFIG_MASK_DIRS.iter().map(|rel| config_base.join(rel)))
        .collect()
}

/// Mask list for the current host. Empty when no home dir is known.
pub fn builtin_mask_roots() -> Vec<PathBuf> {
    let Some(home) = std::env::home_dir() else {
        return Vec::new();
    };
    let xdg_config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    mask_roots_for(&home, xdg_config.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_include_ssh() {
        let home = Path::new("/home/testuser");
        let roots = mask_roots_for(home, None);
        assert!(roots.contains(&home.join(".ssh")));
        assert!(roots.contains(&home.join(".aws")));
        assert!(roots.contains(&home.join(".config/gh")));
    }
}
