//! JSON session persistence under the XDG data dir. Port of
//! `_vendor/embedded-agent/src/session/storage.rs` (session + tool-output
//! paths only; theme/suffix/agent-file helpers are out of scope). Every op
//! has an `_in(base)` variant taking an explicit base dir so tests can use
//! a temp dir instead of the real data dir.

use std::path::{Path, PathBuf};

use super::types::Session;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Base data dir: `$CA_DATA_DIR`, else `$XDG_DATA_HOME/computer-agent`,
/// else `$HOME/.local/share/computer-agent`.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CA_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        let xdg = PathBuf::from(xdg);
        if !xdg.as_os_str().is_empty() {
            return xdg.join("computer-agent");
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match home {
        Some(h) => h.join(".local").join("share").join("computer-agent"),
        None => PathBuf::from("."),
    }
}

pub fn session_dir() -> PathBuf {
    data_dir().join("sessions")
}

pub fn tool_output_dir(session_id: &str) -> PathBuf {
    data_dir()
        .join("tool-outputs")
        .join(safe_path_component(session_id))
}

/// Write atomically: temp file in the same directory, then rename, so a
/// crash mid-write leaves the previous version intact.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn save_session(session: &Session) -> Result<()> {
    save_session_in(&data_dir(), session)
}

pub fn save_session_in(base: &Path, session: &Session) -> Result<()> {
    let dir = base.join("sessions");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", safe_path_component(&session.id)));
    atomic_write(&path, &serde_json::to_string(session)?)?;
    Ok(())
}

pub fn load_session(id: &str) -> Result<Option<Session>> {
    load_session_in(&data_dir(), id)
}

pub fn load_session_in(base: &Path, id: &str) -> Result<Option<Session>> {
    let path = base
        .join("sessions")
        .join(format!("{}.json", safe_path_component(id)));
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&std::fs::read_to_string(
        &path,
    )?)?))
}

pub fn delete_session(id: &str) -> Result<()> {
    delete_session_in(&data_dir(), id)
}

pub fn delete_session_in(base: &Path, id: &str) -> Result<()> {
    let path = base
        .join("sessions")
        .join(format!("{}.json", safe_path_component(id)));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn find_recent_sessions(limit: usize) -> Result<Vec<Session>> {
    find_recent_sessions_in(&data_dir(), limit)
}

pub fn find_recent_sessions_in(base: &Path, limit: usize) -> Result<Vec<Session>> {
    let dir = base.join("sessions");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|e| e == "json"))
        .filter(|e| {
            !e.path().extension().is_some_and(|_| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(".tmp"))
            })
        })
        .map(|e| {
            let mtime = e.metadata().ok().and_then(|m| m.modified().ok());
            (mtime, e.path())
        })
        .collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.0));
    let mut out = Vec::new();
    for (_, path) in entries.iter().take(limit) {
        if let Ok(json) = std::fs::read_to_string(path) {
            if let Ok(session) = serde_json::from_str::<Session>(&json) {
                out.push(session);
            }
        }
    }
    Ok(out)
}

pub fn save_tool_output(session_id: &str, tool_name: &str, output: &str) -> Result<PathBuf> {
    save_tool_output_in(&data_dir(), session_id, tool_name, output)
}

pub fn save_tool_output_in(
    base: &Path,
    session_id: &str,
    tool_name: &str,
    output: &str,
) -> Result<PathBuf> {
    let dir = base
        .join("tool-outputs")
        .join(safe_path_component(session_id));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "{}-{}.txt",
        uuid::Uuid::new_v4(),
        safe_path_component(tool_name)
    ));
    std::fs::write(&path, output)?;
    Ok(path)
}

fn safe_path_component(value: &str) -> String {
    let safe: String = value
        .chars()
        .take(64)
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "tool".to_string()
    } else {
        safe
    }
}
