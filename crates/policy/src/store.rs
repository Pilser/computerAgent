//! Session allowlist: in-memory `Approve` grants + plain-text persist.
//!
//! Format is one `tool\tpattern` line per grant. Matching delegates to
//! [`Pattern`](crate::pattern::Pattern) so allowlist and rules agree.

use crate::decision::PolicyError;
use crate::pattern::Pattern;

/// Session-scoped `(tool, pattern)` grants. Never bypasses deny rules —
/// the checker evaluates denies before consulting this store.
#[derive(Debug, Default, Clone)]
pub struct SessionAllowlist {
    entries: Vec<(String, String)>,
}

impl SessionAllowlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, tool: &str, pattern: &str) {
        self.entries.push((tool.to_string(), pattern.to_string()));
        // Mirror vendor: also store the absolutized form for path tools.
        if is_path_tool(tool) {
            let abs = absolutize(pattern);
            if abs != pattern {
                self.entries.push((tool.to_string(), abs));
            }
        }
    }

    pub fn load(&mut self, entries: &[(String, String)]) {
        for (tool, pat) in entries {
            self.add(tool, pat);
        }
    }

    pub fn is_allowed(&self, tool: &str, input: &str) -> bool {
        self.entries.iter().any(|(t, p)| {
            t == tool
                && (Pattern::glob(p).matches(input) || Pattern::glob(p).matches(&absolutize(input)))
        })
    }

    pub fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Persist to `path` as `tool\tpattern` lines.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), PolicyError> {
        let mut out = String::new();
        for (tool, pat) in &self.entries {
            out.push_str(tool);
            out.push('\t');
            out.push_str(&pat.replace('\n', " "));
            out.push('\n');
        }
        std::fs::write(path, out).map_err(|e| PolicyError::Io(e.to_string()))
    }

    /// Load `tool\tpattern` lines; missing file yields an empty store.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, PolicyError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(PolicyError::Io(e.to_string())),
        };
        let mut store = Self::new();
        for line in text.lines() {
            let (tool, pat) = match line.split_once('\t') {
                Some(p) => p,
                None => continue,
            };
            if !tool.is_empty() && !pat.is_empty() {
                // Push raw (no absolutize fan-out) to keep save/load stable.
                store.entries.push((tool.to_string(), pat.to_string()));
            }
        }
        Ok(store)
    }
}

fn is_path_tool(tool: &str) -> bool {
    matches!(tool, "read" | "write" | "edit" | "list_dir")
}

fn absolutize(path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(p).to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_grants_matching_input() {
        let mut s = SessionAllowlist::new();
        assert!(s.is_empty());
        s.add("bash", "ls *");
        assert!(!s.is_empty());
        assert!(s.is_allowed("bash", "ls foo"));
        assert!(!s.is_allowed("bash", "rm foo"));
        assert!(!s.is_allowed("read", "ls foo"));
    }

    #[test]
    fn file_round_trip() {
        let dir = std::env::temp_dir().join("ca-policy-test-allowlist");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("allowlist.tsv");
        let mut s = SessionAllowlist::new();
        s.add("read", "/tmp/**");
        s.save_to_file(&path).unwrap();
        let back = SessionAllowlist::load_from_file(&path).unwrap();
        assert!(back.is_allowed("read", "/tmp/a/b"));
        assert!(!back.is_allowed("read", "/etc/passwd"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_loads_empty() {
        let s = SessionAllowlist::load_from_file(std::path::Path::new(
            "/nonexistent-ca-allowlist-xyz.tsv",
        ))
        .unwrap();
        assert!(s.is_empty());
    }
}
