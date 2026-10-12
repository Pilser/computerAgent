//! Memory store paths: `Mem { root, project, today }`, slug, open/list/read.
//! Port of the `Mem` core in `_vendor/embedded-agent/src/extras/memory/mod.rs`
//! (path/layout half; write/edit/search/inject live in sibling files).

use std::fs;
use std::path::{Path, PathBuf};

use super::{is_daily_log_stem, truncate_cjk, MAX_INJECT_BYTES};

/// Filesystem-safe, collision-resistant slug for a project path:
/// `<sanitized-basename>-<8 hex of full-path hash>`. FNV-1a 64-bit (stable
/// across Rust versions and platforms) so slugs survive compiler upgrades.
pub fn project_slug(path: &Path) -> String {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &byte in path.as_os_str().as_encoded_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    let short = hash as u32;
    let base = path.file_name().and_then(|s| s.to_str()).unwrap_or("root");
    let mut slug: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    slug.truncate(40);
    if slug.is_empty() {
        slug.push_str("root");
    }
    format!("{slug}-{short:08x}")
}

/// A daily log selected for injection: the file's date stem + raw content.
#[derive(Debug, Clone)]
pub(crate) struct DailyLog {
    pub(crate) date: String,
    pub(crate) content: String,
}

/// Long-term memory handle. `MEMORY.md` is global; scratchpad/daily/notes are
/// scoped by `project` so different projects do not pollute each other.
#[derive(Debug, Clone)]
pub struct Mem {
    pub root: PathBuf,
    pub project: String,
    pub today: String,
}

impl Mem {
    /// Open the store at `<data_dir>/agent/memory/` (same base as session
    /// storage), scoping per-project files by the current working directory.
    pub fn open() -> Self {
        let root = crate::session::storage::data_dir()
            .join("agent")
            .join("memory");
        let project = std::env::current_dir()
            .map(|p| project_slug(&p))
            .unwrap_or_else(|_| "default".to_string());
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        Mem {
            root,
            project,
            today,
        }
    }

    /// Test constructor: explicit root/project/today, no env access.
    pub fn open_in(root: PathBuf, project: &str, today: &str) -> Self {
        Mem {
            root,
            project: project.to_string(),
            today: today.to_string(),
        }
    }

    /// Global facts file, shared across projects.
    pub fn memory_md(&self) -> PathBuf {
        self.root.join("MEMORY.md")
    }

    pub(crate) fn project_dir(&self) -> PathBuf {
        self.root.join("projects").join(&self.project)
    }

    /// Per-project open checklist.
    pub fn scratchpad(&self) -> PathBuf {
        self.project_dir().join("SCRATCHPAD.md")
    }

    pub(crate) fn daily_dir(&self) -> PathBuf {
        self.project_dir().join("daily")
    }

    pub(crate) fn notes_dir(&self) -> PathBuf {
        self.project_dir().join("notes")
    }

    /// Enumerate every `.md` memory file: global `MEMORY.md` plus the current
    /// project's notes and daily logs, sorted. Non-`.md` files (notably
    /// `.bak` backups and `.tmp` crash leftovers) are excluded.
    pub fn list(&self) -> Vec<String> {
        let mut names = Vec::new();
        for dir in [self.root.clone(), self.notes_dir(), self.daily_dir()] {
            if let Ok(rd) = fs::read_dir(&dir) {
                for e in rd.flatten() {
                    if e.path().extension().is_some_and(|x| x == "md") {
                        names.push(e.path().display().to_string());
                    }
                }
            }
        }
        names.sort();
        names
    }

    /// Path for a daily log. No sanitization: callers must validate `date`
    /// with [`super::is_safe_daily_name`] first.
    pub fn daily_file(&self, date: &str) -> PathBuf {
        self.daily_dir().join(format!("{date}.md"))
    }

    /// Sanitize a note name so it can never escape `notes/`: no slashes,
    /// backslashes, or dots. Trims a trailing `.md` so `x` and `x.md` agree.
    pub fn note_path(&self, name: &str) -> Option<PathBuf> {
        let stem = name.trim().trim_end_matches(".md");
        if stem.is_empty() || stem.contains(['/', '\\', '.']) {
            return None;
        }
        Some(self.notes_dir().join(format!("{stem}.md")))
    }

    /// Read a memory file for the model, capped so a long file cannot flood
    /// context. Missing/empty reads as `(empty)`.
    pub fn read_capped(p: &Path) -> String {
        match fs::read_to_string(p) {
            Ok(s) if s.is_empty() => "(empty)".to_string(),
            Ok(s) => truncate_cjk(&s, MAX_INJECT_BYTES, "\n…[memory truncated]"),
            Err(_) => "(empty)".to_string(),
        }
    }

    /// Up to the two most recent non-empty daily logs, newest first. Scans
    /// `daily/`, sorts `YYYY-MM-DD.md` filenames descending, and skips
    /// empty/whitespace-only files rather than stopping at the first one.
    pub(crate) fn recent_daily_logs(&self) -> Vec<DailyLog> {
        let mut files: Vec<PathBuf> = match fs::read_dir(self.daily_dir()) {
            Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
            Err(_) => return Vec::new(),
        };
        files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        let mut logs = Vec::new();
        for path in files {
            if logs.len() >= 2 {
                break;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(date) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if !is_daily_log_stem(date) {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            if content.trim().is_empty() {
                continue;
            }
            logs.push(DailyLog {
                date: date.to_string(),
                content,
            });
        }
        logs
    }
}

#[cfg(test)]
mod tests {
    use super::super::is_safe_daily_name;
    use super::*;

    #[test]
    fn slug_is_stable_and_distinct() {
        let a = project_slug(Path::new("/home/u/my project!"));
        let b = project_slug(Path::new("/home/u/my project!"));
        assert_eq!(a, b);
        assert!(a.starts_with("my-project--") || a.starts_with("my-project-"));
        // Same basename, different parent: distinct slugs.
        let c = project_slug(Path::new("/other/my project!"));
        assert_ne!(a, c);
    }

    #[test]
    fn note_path_sanitizes() {
        let m = Mem::open_in(PathBuf::from("/tmp/x"), "p", "2026-01-01");
        assert_eq!(
            m.note_path("ideas"),
            Some(PathBuf::from("/tmp/x/projects/p/notes/ideas.md"))
        );
        // Trailing .md is trimmed so both spellings agree.
        assert_eq!(m.note_path("ideas"), m.note_path("ideas.md"));
        assert_eq!(m.note_path("../../.."), None);
        assert_eq!(m.note_path("a/b"), None);
        assert_eq!(m.note_path("a\\b"), None);
        assert_eq!(m.note_path("a.b"), None);
        assert_eq!(m.note_path(""), None);
        assert_eq!(m.note_path("   "), None);
    }

    #[test]
    fn daily_name_gate() {
        assert!(is_safe_daily_name("2026-10-12"));
        assert!(!is_safe_daily_name(""));
        assert!(!is_safe_daily_name("../../secret"));
        assert!(!is_safe_daily_name("2026/10/12"));
        assert!(!is_safe_daily_name("today.md"));
    }
}
