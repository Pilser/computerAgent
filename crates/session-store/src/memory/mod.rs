//! Long-term memory: slimmed port of
//! `_vendor/embedded-agent/src/extras/memory/mod.rs`.
//!
//! Layout under `<base>/agent/memory/` (same base as session storage,
//! `$CA_DATA_DIR/agent/memory/`): global `MEMORY.md`, per-project
//! `projects/<slug>/{SCRATCHPAD.md,daily/*.md,notes/*.md}`.
//!
//! Caps (see `AGENTS.md` §4.6): 32 KiB inject / 64 KiB single write.

pub mod edit;
pub mod inject;
pub mod search;
pub mod store;
pub mod write;

pub use search::{SearchHit, SearchResults};
pub use store::{project_slug, Mem};
pub use write::{WriteMode, WriteTarget};

use std::path::Path;

/// Hard cap on injected/returned memory: a context-window guard, not a
/// memory-usage one. Files are expected to be small.
pub const MAX_INJECT_BYTES: usize = 32 * 1024;

/// Hard cap on content accepted by a single write call. Longer content is
/// truncated (with a warning in the response), not rejected.
pub const MAX_WRITE_BYTES: usize = 64 * 1024;

/// Write atomically: temp file first, then rename, so readers see either the
/// old content or the new, never a partial write.
pub(crate) fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Single-version backup (`MEMORY.md` -> `MEMORY.bak`, overwriting any prior
/// `.bak`). No-op when `path` does not exist yet.
fn backup_file(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::copy(path, path.with_extension("bak"))?;
    }
    Ok(())
}

/// Back up `path`, returning a warning suffix on failure (empty on success).
/// Fail-open: the mutation still proceeds, but the response must say there is
/// no `.bak` to undo it.
pub(crate) fn backup_warning(path: &Path) -> String {
    match backup_file(path) {
        Ok(()) => String::new(),
        Err(e) => format!(" (warning: backup failed, no .bak written: {e})"),
    }
}

/// Normalize a line for long-term append dedup: trim, collapse every run of
/// Unicode whitespace to one ASCII space, preserving case. Whitespace-only
/// lines normalize to empty and are never dedup keys.
pub(crate) fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A daily-log name is caller-supplied, so validate before splicing into a
/// path: ASCII digits and hyphens only (enough for `YYYY-MM-DD`), which
/// forbids `.`, `/`, and `\`.
pub(crate) fn is_safe_daily_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_digit() || c == '-')
}

/// True if `s` is exactly `YYYY-MM-DD` shaped, so only real daily-log files
/// (never a stray `.tmp` or an unrelated `.md`) are scanned and interpolated
/// into the injected block.
pub(crate) fn is_daily_log_stem(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Truncate `s` to at most `max` bytes on a UTF-8 char boundary, appending
/// `marker`. Plain `String::truncate` panics mid-character (e.g. on CJK).
pub(crate) fn truncate_cjk(s: &str, max: usize, marker: &str) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut cut = max;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push_str(marker);
    out
}

/// Byte-prefix of `s` on a char boundary (no marker): the write-path
/// truncation primitive.
pub(crate) fn truncate_to_bytes(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut cut = max;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    &s[..cut]
}
