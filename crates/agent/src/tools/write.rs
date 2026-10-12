//! `write` tool: create a new file, creating parent dirs first.
//!
//! Port of vendor `agent/tools/write.rs` minus LSP sync (no `lsp` feature in
//! v1) and with policy gating instead of `PermCheck` + ask channel. Fails
//! when the file exists (targeted edits arrive with the `edit` tool later).

use serde::{Deserialize, Serialize};

use crate::error::ToolError;
use crate::tools::read::untrack_read_path;
use crate::tools::{check_policy_path, expand_tilde, SharedPolicy, Tool};

pub const DEFAULT_MAX_TEXT_SIZE: u64 = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct WriteArgs {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub note: Option<String>,
}

pub struct WriteTool {
    pub policy: Option<SharedPolicy>,
    pub max_text_file_size: u64,
}

impl WriteTool {
    pub fn new(policy: Option<SharedPolicy>, max_text_file_size: Option<u64>) -> Self {
        Self {
            policy,
            max_text_file_size: max_text_file_size.unwrap_or(DEFAULT_MAX_TEXT_SIZE),
        }
    }
}

impl Tool for WriteTool {
    const NAME: &'static str = "write";
    type Args = WriteArgs;

    fn description(&self) -> String {
        "Create a new file with the given content. Fails if the file already exists — use edit for existing files. Automatically creates parent directories.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file (relative or absolute)" },
                "content": { "type": "string", "description": "Content to write to the file" },
                "note": { "type": "string", "description": "Short human note shown in the UI next to this action (optional)" }
            },
            "required": ["path", "content"]
        })
    }

    async fn call(&self, args: WriteArgs) -> Result<String, ToolError> {
        let expanded = expand_tilde(&args.path);
        // Policy FIRST, before any sys-tools call.
        check_policy_path(&self.policy, Self::NAME, &expanded)?;

        let path = std::path::Path::new(&expanded);
        if path.exists() {
            return Err(ToolError::Msg(format!(
                "File '{expanded}' already exists. Use edit for targeted changes, or delete and recreate if a full rewrite is needed."
            )));
        }
        // `parent()` is `Some("")` for bare filenames; creating that would
        // fail, so skip empty parents.
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let bytes = args.content.len();
        if bytes as u64 > self.max_text_file_size {
            return Err(ToolError::Msg(format!(
                "File too large ({bytes} bytes). Maximum allowed file size is {} bytes.",
                self.max_text_file_size
            )));
        }
        // NOTE: vendor does an atomic write; sys-tools v1 `write_text` is a
        // plain write. Atomicity can move into sys-tools later.
        computer_agent_sys_tools::fs::write_text(path, &args.content).await?;
        // A rewritten path invalidates the repeated-read tracker (port).
        untrack_read_path(&expanded);
        Ok(format!("Written {bytes} bytes to {expanded}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::open_policy;
    use crate::tools::read::{clear_read_tracker, ReadArgs};
    use crate::tools::{ReadTool, Tool};

    fn tmp(name: &str) -> std::path::PathBuf {
        std::path::Path::new("/tmp/opencode")
            .join(format!("agent-write-{}-{name}", std::process::id()))
    }

    #[tokio::test]
    async fn round_trip_and_dir_create() {
        let file = tmp("roundtrip").join("deep/nested/out.txt");
        let _ = std::fs::remove_dir_all(tmp("roundtrip"));
        let tool = WriteTool::new(Some(open_policy()), None);
        let out = tool
            .call(WriteArgs {
                path: file.to_string_lossy().into(),
                content: "hello agent".into(),
                note: None,
            })
            .await
            .unwrap();
        assert!(out.contains("Written 11 bytes"), "{out}");
        let back = computer_agent_sys_tools::fs::read_text(&file)
            .await
            .unwrap();
        assert_eq!(back, "hello agent");
        std::fs::remove_dir_all(tmp("roundtrip")).unwrap();
    }

    #[tokio::test]
    async fn existing_file_fails() {
        let dir = tmp("exists");
        let file = dir.join("e.txt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, "old").unwrap();
        let tool = WriteTool::new(Some(open_policy()), None);
        let err = tool
            .call(WriteArgs {
                path: file.to_string_lossy().into(),
                content: "new".into(),
                note: None,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn write_untracks_read_path() {
        // Read-then-write-then-read the same window must not trip the
        // repeated-read guard (vendor `untrack_read_path` contract).
        let dir = tmp("untrack");
        let file = dir.join("u.txt");
        let path = file.to_string_lossy().into_owned();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, "v1").unwrap();
        clear_read_tracker();

        let read = ReadTool::new(Some(open_policy()), None, None);
        let args = || ReadArgs {
            path: path.clone(),
            offset: None,
            limit: None,
            note: None,
        };
        read.call(args()).await.unwrap();
        std::fs::remove_file(&file).unwrap();
        WriteTool::new(Some(open_policy()), None)
            .call(WriteArgs {
                path: path.clone(),
                content: "v2".into(),
                note: None,
            })
            .await
            .unwrap();
        assert!(read.call(args()).await.is_ok());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
