//! Memory edits: unique-substring replacement + whole-note deletion.

use std::fs;

use super::store::Mem;
use super::write::WriteTarget;
use super::{atomic_write, backup_warning, is_safe_daily_name};

impl Mem {
    /// Strict, unique-substring replacement in a memory file.
    ///
    /// `old_str` must occur EXACTLY once (0 or >1 matches is a hard error
    /// naming the count; nothing is written). The match is replaced with
    /// `new_str` verbatim, no newline cleanup; `new_str=""` deletes exactly
    /// the matched bytes. No fuzzy fallback.
    ///
    /// `old_str: None` deletes a whole note (`target=Note`, `name=<stem>`);
    /// omitting it for any other target is a hard error touching nothing.
    pub fn edit(
        &self,
        target: WriteTarget,
        name: Option<&str>,
        old_str: Option<&str>,
        new_str: &str,
    ) -> std::io::Result<String> {
        let path = match target {
            WriteTarget::LongTerm => self.memory_md(),
            WriteTarget::Scratchpad => self.scratchpad(),
            WriteTarget::Daily => {
                // Honor a caller-supplied date, defaulting to today; validate
                // first since `daily_file` does no sanitization of its own.
                let date = name.unwrap_or(&self.today);
                if !is_safe_daily_name(date) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid daily date name (expected YYYY-MM-DD: digits and hyphens only)",
                    ));
                }
                self.daily_file(date)
            }
            WriteTarget::Note => match name.and_then(|n| self.note_path(n)) {
                Some(p) => p,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid note name (no slashes or dots)",
                    ));
                }
            },
        };
        match old_str {
            Some(old) => {
                let current = fs::read_to_string(&path)?;
                let count = current.match_indices(old).count();
                if count != 1 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "old_str matched {count} time(s) in {}; it must match exactly once",
                            path.display()
                        ),
                    ));
                }
                let pos = current
                    .find(old)
                    .expect("match_indices count == 1 guarantees a match");
                let mut updated = current;
                updated.replace_range(pos..pos + old.len(), new_str);
                // Back up curated files before in-place mutation; daily/note
                // edits are low-risk and out of scope.
                let mut warn = String::new();
                if matches!(target, WriteTarget::LongTerm | WriteTarget::Scratchpad) {
                    warn = backup_warning(&path);
                }
                atomic_write(&path, &updated)?;
                Ok(format!("Edited {}{warn}", path.display()))
            }
            None => {
                if !matches!(target, WriteTarget::Note) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "memory_edit without old_str deletes a whole note; \
it is only allowed for target=note, not long_term/scratchpad/daily",
                    ));
                }
                // `path` is the sanitized note path. Removing a missing note
                // surfaces NotFound rather than a silent Ok. Back up first so
                // the deletion is recoverable.
                let warn = backup_warning(&path);
                fs::remove_file(&path)?;
                Ok(format!("Deleted {}{warn}", path.display()))
            }
        }
    }
}
