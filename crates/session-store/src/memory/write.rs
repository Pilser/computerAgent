//! Memory writes: append/overwrite, long-term dedup, atomic write + `.bak`.

use std::collections::HashSet;
use std::fs;

use super::store::Mem;
use super::{atomic_write, backup_warning, normalize_line, truncate_to_bytes, MAX_WRITE_BYTES};

/// Where a write lands. `LongTerm` is the global `MEMORY.md`; the rest are
/// per-project files (see [`Mem`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTarget {
    LongTerm,
    Scratchpad,
    Daily,
    Note,
}

/// `Append` adds (newline-joined); `Overwrite` replaces the whole file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Append,
    Overwrite,
}

impl Mem {
    /// Write `content` to `target`. Content over [`MAX_WRITE_BYTES`] is
    /// truncated with a warning, not rejected. Long-term appends dedupe
    /// whitespace-insensitively (see below); overwrites of curated files
    /// (`LongTerm`/`Scratchpad`) back up the prior version to `.bak`.
    pub fn write(
        &self,
        target: WriteTarget,
        content: &str,
        mode: WriteMode,
        name: Option<&str>,
    ) -> std::io::Result<String> {
        let original_len = content.len();
        let content = truncate_to_bytes(content, MAX_WRITE_BYTES);
        let path = match target {
            WriteTarget::LongTerm => self.memory_md(),
            WriteTarget::Scratchpad => self.scratchpad(),
            WriteTarget::Daily => {
                fs::create_dir_all(self.daily_dir())?;
                self.daily_file(&self.today)
            }
            WriteTarget::Note => {
                fs::create_dir_all(self.notes_dir())?;
                match name.and_then(|n| self.note_path(n)) {
                    Some(p) => p,
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "invalid note name (no slashes or dots)",
                        ));
                    }
                }
            }
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut appended_len = content.len();
        let mut dedup_note = String::new();
        let mut nothing_written: Option<usize> = None;
        let mut backup_note = String::new();
        match mode {
            WriteMode::Overwrite => {
                if matches!(target, WriteTarget::LongTerm | WriteTarget::Scratchpad) {
                    backup_note = backup_warning(&path);
                }
                atomic_write(&path, content)?;
            }
            WriteMode::Append => {
                // Long-term memory is curated one-fact-per-line: drop lines
                // whose normalized form repeats within this batch or already
                // exists in MEMORY.md. Blank lines are never dedup keys (they
                // carry no fact) and are kept verbatim. Scratchpad/daily/note
                // appends stay byte-identical (no dedup).
                let mut append_content = content.to_string();
                if matches!(target, WriteTarget::LongTerm) {
                    let existing = fs::read_to_string(&path).unwrap_or_default();
                    let mut seen: HashSet<String> = existing
                        .lines()
                        .map(normalize_line)
                        .filter(|n| !n.is_empty())
                        .collect();
                    let mut kept: Vec<&str> = Vec::new();
                    let mut skipped = 0usize;
                    for line in content.split('\n') {
                        let norm = normalize_line(line);
                        if norm.is_empty() {
                            kept.push(line);
                            continue;
                        }
                        // `insert` is false when already present: covers both
                        // prior file content and earlier batch lines.
                        if seen.insert(norm) {
                            kept.push(line);
                        } else {
                            skipped += 1;
                        }
                    }
                    if skipped > 0 {
                        let survived = kept.iter().any(|l| !normalize_line(l).is_empty());
                        if survived {
                            append_content = kept.join("\n");
                            dedup_note = format!(" (skipped {skipped} duplicate line(s))");
                        } else {
                            // Nothing meaningful survived: leave the file
                            // untouched and report it.
                            nothing_written = Some(skipped);
                        }
                    }
                }
                if nothing_written.is_none() {
                    // Files are small; read-modify-write is clearer than
                    // seeking to inspect the last byte.
                    let mut prev = fs::read_to_string(&path).unwrap_or_default();
                    if !prev.is_empty() && !prev.ends_with('\n') {
                        prev.push('\n');
                    }
                    prev.push_str(&append_content);
                    if !prev.ends_with('\n') {
                        prev.push('\n');
                    }
                    atomic_write(&path, &prev)?;
                    appended_len = append_content.len();
                }
            }
        }
        if let Some(n) = nothing_written {
            return Ok(format!(
                "Nothing written to {}: all {n} line(s) were duplicates",
                path.display()
            ));
        }
        let mut msg = format!(
            "Wrote {appended_len} bytes to {}{dedup_note}{backup_note}",
            path.display()
        );
        if original_len > content.len() {
            msg.push_str(&format!(
                " (truncated from {original_len} bytes to {} byte cap)",
                MAX_WRITE_BYTES
            ));
        }
        Ok(msg)
    }

    /// Append a timestamped entry to today's daily log. Used by the
    /// pre-compaction flush so the rolling summary survives compaction
    /// deterministically.
    pub fn append_daily(&self, heading: &str, body: &str) -> std::io::Result<()> {
        let stamp = chrono::Local::now().format("%H:%M").to_string();
        let entry = format!("\n### {stamp} — {heading}\n{}\n", body.trim());
        self.write(WriteTarget::Daily, &entry, WriteMode::Append, None)?;
        Ok(())
    }
}
