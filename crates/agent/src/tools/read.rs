//! `read` tool: paged text reads with a deny-repeated-read tracker.
//!
//! Port of vendor `agent/tools/read.rs`, trimmed: plain-text output only (no
//! `EditSystem::Hashedit` CRC tags — no config crate in v1), permission via
//! our policy checker instead of `PermCheck` + ask channel.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::ToolError;
use crate::tools::{check_policy_path, expand_tilde, SharedPolicy, Tool};

pub const DEFAULT_MAX_TEXT_SIZE: u64 = 1024 * 1024;
pub const DEFAULT_MAX_LINES: usize = 2000;

static READ_TRACKER: Mutex<Vec<(String, usize, usize)>> = Mutex::new(Vec::new());
static DENY_REPEATED_READS: Mutex<bool> = Mutex::new(true);

/// Port of vendor `track_read`: repeat reads of the same `(path, offset,
/// limit)` window are blocked until the file changes (see
/// [`untrack_read_path`]). Returns the block message when blocked.
pub(crate) fn track_read(path: &str, offset: usize, limit: usize) -> Option<String> {
    if !deny_repeated_reads() {
        return None;
    }
    let mut tracker = READ_TRACKER.lock().unwrap_or_else(|e| e.into_inner());
    let key = (path.to_string(), offset, limit);
    if tracker.contains(&key) {
        let end = offset + limit;
        Some(format!(
            "read blocked: {path} (lines {}-{}) was already read and has not been modified since. Use the previous result or read a different section.",
            // `offset` is 0-based here; display is 1-based.
            offset + 1,
            if end > 0 { end } else { offset + 1 }
        ))
    } else {
        tracker.push(key);
        None
    }
}

/// Port of vendor `untrack_read_path`: `write` calls this so a rewritten file
/// can be read again.
pub(crate) fn untrack_read_path(path: &str) {
    READ_TRACKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|(p, _, _)| p != path);
}

pub(crate) fn set_deny_repeated_reads(v: bool) {
    *DENY_REPEATED_READS
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = v;
}

fn deny_repeated_reads() -> bool {
    *DENY_REPEATED_READS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
pub(crate) fn clear_read_tracker() {
    READ_TRACKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReadArgs {
    pub path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub note: Option<String>,
}

pub struct ReadTool {
    pub policy: Option<SharedPolicy>,
    pub max_text_file_size: u64,
    pub max_lines: usize,
}

impl ReadTool {
    pub fn new(
        policy: Option<SharedPolicy>,
        max_text_file_size: Option<u64>,
        max_lines: Option<usize>,
    ) -> Self {
        Self {
            policy,
            max_text_file_size: max_text_file_size.unwrap_or(DEFAULT_MAX_TEXT_SIZE),
            max_lines: max_lines.unwrap_or(DEFAULT_MAX_LINES),
        }
    }
}

impl Tool for ReadTool {
    const NAME: &'static str = "read";
    type Args = ReadArgs;

    fn description(&self) -> String {
        format!(
            "Read the contents of a file. Supports text files. Defaults to first {} lines. Use offset/limit for large files.",
            self.max_lines
        )
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file (relative or absolute)" },
                "offset": { "type": "integer", "description": "Line number to start from (1-indexed)" },
                "limit": { "type": "integer", "description": "Maximum number of lines to read" },
                "note": { "type": "string", "description": "Short human note shown in the UI next to this action (optional)" }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, args: ReadArgs) -> Result<String, ToolError> {
        let path = expand_tilde(&args.path);
        // 1-based UI offset -> 0-based window. Port of vendor `call`.
        let offset = args.offset.unwrap_or(1).saturating_sub(1);
        let limit = args.limit.unwrap_or(self.max_lines);
        // Policy FIRST, before any sys-tools call.
        check_policy_path(&self.policy, Self::NAME, &path)?;

        if let Some(msg) = track_read(&path, offset, limit) {
            return Err(ToolError::Msg(msg));
        }

        // Size precheck via metadata only (cheap stat, not a read).
        let size = tokio::fs::metadata(&path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        if size > self.max_text_file_size {
            return Err(ToolError::Msg(format!(
                "File too large ({size} bytes). Maximum allowed file size is {} bytes.",
                self.max_text_file_size
            )));
        }
        let content = computer_agent_sys_tools::fs::read_text(std::path::Path::new(&path)).await?;
        let total_lines = content.lines().count();
        let (start, end) = read_bounds(offset, limit, total_lines);
        let excerpt: Vec<&str> = content.lines().skip(start).take(end - start).collect();

        let mut info = format!(
            "File: {path} ({total_lines} lines total, showing lines {}-{})\n\n{}",
            display_start(start, total_lines),
            end,
            excerpt.join("\n")
        );
        if end < total_lines {
            let remaining = total_lines - end;
            info.push_str(&format!(
                "\n\n[truncated after {} lines — {remaining} more lines (lines {}-{total_lines}); re-call with offset/limit to see more]",
                end - start,
                end + 1,
            ));
        }
        Ok(info)
    }
}

fn read_bounds(offset: usize, limit: usize, total_lines: usize) -> (usize, usize) {
    let start = offset.min(total_lines);
    let end = start.saturating_add(limit).min(total_lines);
    (start, end)
}

fn display_start(start: usize, total_lines: usize) -> usize {
    if total_lines == 0 {
        0
    } else {
        start + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::open_policy;
    use computer_agent_policy::decision::SecurityMode;
    use computer_agent_policy::PermissionChecker;
    use std::sync::{Arc, Mutex};

    fn tmp(name: &str) -> std::path::PathBuf {
        std::path::Path::new("/tmp/opencode")
            .join(format!("agent-read-{}-{name}", std::process::id()))
    }

    fn numbered(n: usize) -> String {
        (1..=n)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn bounds_helpers_match_vendor() {
        assert_eq!(read_bounds(20, 10, 5), (5, 5));
        assert_eq!(read_bounds(2, 3, 10), (2, 5));
        assert_eq!(display_start(0, 0), 0);
        assert_eq!(display_start(4, 10), 5);
    }

    #[tokio::test]
    async fn paging_returns_requested_window() {
        let dir = tmp("paging");
        let file = dir.join("ten.txt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, numbered(10)).unwrap();
        clear_read_tracker();

        let tool = ReadTool::new(Some(open_policy()), None, None);
        let out = tool
            .call(ReadArgs {
                path: file.to_string_lossy().into(),
                offset: Some(4),
                limit: Some(3),
                note: None,
            })
            .await
            .unwrap();
        assert!(out.contains("10 lines total, showing lines 4-6"), "{out}");
        assert!(out.contains("line4\nline5\nline6"));
        assert!(out.contains("truncated after 3 lines"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn repeat_read_of_same_window_blocks() {
        let dir = tmp("repeat");
        let file = dir.join("r.txt");
        let path = file.to_string_lossy().into_owned();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, numbered(5)).unwrap();
        clear_read_tracker();

        let tool = ReadTool::new(Some(open_policy()), None, None);
        let args = || ReadArgs {
            path: path.clone(),
            offset: Some(1),
            limit: Some(5),
            note: None,
        };
        tool.call(args()).await.unwrap();
        let err = tool.call(args()).await.unwrap_err();
        assert!(err.to_string().contains("already read"), "{err}");

        // A different window is still fine.
        let other = tool
            .call(ReadArgs {
                path: path.clone(),
                offset: Some(2),
                limit: Some(2),
                note: None,
            })
            .await;
        assert!(other.is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn policy_denied_read_blocks_before_io() {
        let mut c = PermissionChecker::new(SecurityMode::Standard);
        c.deny("read", "/blocked/**");
        let policy = Arc::new(Mutex::new(c));
        let tool = ReadTool::new(Some(policy), None, None);
        let err = tool
            .call(ReadArgs {
                path: "/blocked/secret.txt".into(),
                offset: None,
                limit: None,
                note: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::Denied(_)), "{err}");
    }
}
