//! Append-only chat history as JSON lines. The vendor keeps a single JSON
//! array (`chat_history.json`) rewritten on every append; JSONL scales
//! better (O(1) append, no full-file rewrite) and tolerates a corrupt line
//! without losing the whole history.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::storage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatHistoryEntry {
    pub content: String,
    pub timestamp: String,
}

impl ChatHistoryEntry {
    pub fn now(content: &str) -> Self {
        Self {
            content: content.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub fn history_path() -> PathBuf {
    storage::data_dir().join("chat_history.jsonl")
}

fn history_path_in(base: &Path) -> PathBuf {
    base.join("chat_history.jsonl")
}

pub fn append_entry(entry: &ChatHistoryEntry) -> storage::Result<()> {
    append_entry_in(&storage::data_dir(), entry)
}

/// Append one JSON line; creates parent dirs on first use.
pub fn append_entry_in(base: &Path, entry: &ChatHistoryEntry) -> storage::Result<()> {
    use std::io::Write as _;
    let path = history_path_in(base);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub fn load_history() -> storage::Result<Vec<ChatHistoryEntry>> {
    load_history_in(&storage::data_dir())
}

/// Load all entries; blank lines are skipped and malformed lines are
/// skipped (one bad line must not discard the whole history).
pub fn load_history_in(base: &Path) -> storage::Result<Vec<ChatHistoryEntry>> {
    let path = history_path_in(base);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<ChatHistoryEntry>(line) {
            out.push(entry);
        }
    }
    Ok(out)
}
