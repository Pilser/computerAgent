//! Persisted session types. Port of `_vendor/embedded-agent/src/session/mod.rs`
//! minus UI/runtime-only fields (`git_branch`, `git_status`,
//! `show_cost_always`, `reasoning_enabled`, `pending_media`, prompt refs):
//! everything stored here serializes to the session JSON file, except the
//! two runtime fields marked `#[serde(skip)]`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    ToolCall,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: MessageRole,
    pub content: String,
    pub estimated_tokens: u64,
    #[serde(default)]
    pub tool: Option<ToolRecord>,
    /// Turn id that produced this message; `None` for messages added
    /// outside any turn (e.g. compaction recaps).
    #[serde(default)]
    pub turn: Option<String>,
}

/// Structured counterpart to a `ToolCall`/`ToolResult` message's display
/// summary in `content`. `Call` carries the complete argument JSON;
/// `Result` carries linkage + truncation metadata only (result text stays
/// solely in `content`). Untagged: the variants are structurally disjoint
/// (`Call` has `id`/`args`, `Result` has `call_id`/`truncated`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolRecord {
    Call {
        id: u64,
        name: String,
        args: serde_json::Value,
    },
    Result {
        call_id: u64,
        name: String,
        truncated: bool,
        full_output_path: Option<String>,
    },
}

/// Single-step restore point captured before a rewind so the truncation
/// can be undone with [`Session::redo`]. Runtime only, not persisted.
#[derive(Debug, Clone)]
pub struct RewindUndo {
    pub(crate) messages: Vec<SessionMessage>,
    pub(crate) total_estimated_tokens: u64,
    pub(crate) calibrated_tokens: u64,
    pub(crate) calibrated_msg_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compaction {
    pub summary: String,
    pub first_kept_index: usize,
    pub summarized_count: usize,
    pub token_savings: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub messages: Vec<SessionMessage>,
    pub compactions: Vec<Compaction>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub total_input_tokens: u64,
    #[serde(default)]
    pub total_output_tokens: u64,
    #[serde(default)]
    pub total_cached_input_tokens: u64,
    #[serde(default)]
    pub total_cache_creation_input_tokens: u64,
    #[serde(default)]
    pub total_cost: f64,
    #[serde(default)]
    pub total_estimated_tokens: u64,
    #[serde(default)]
    pub calibrated_tokens: u64,
    #[serde(default)]
    pub calibrated_msg_count: usize,
    #[serde(default)]
    pub input_token_cost: f64,
    #[serde(default)]
    pub output_token_cost: f64,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub working_dir: String,
    /// Counter stamping `ToolRecord::Call.id`, persisted so ids stay unique
    /// across a resume instead of restarting at 0. Old files predate tool
    /// records, so defaulting to 0 on load is safe.
    #[serde(default)]
    pub next_tool_call_id: u64,
    /// Fixed request overhead (system prompt, tool schemas, context files)
    /// that never lives in `messages`. Recomputed at runtime, not persisted.
    #[serde(skip)]
    pub overhead_tokens: u64,
    /// Restore point for the most recent rewind. Runtime only, not persisted.
    #[serde(skip)]
    pub rewind_undo: Option<RewindUndo>,
}

impl Session {
    pub fn new(provider: &str, model: &str, context_window: u64, name: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            messages: Vec::new(),
            compactions: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cached_input_tokens: 0,
            total_cache_creation_input_tokens: 0,
            total_cost: 0.0,
            total_estimated_tokens: 0,
            calibrated_tokens: 0,
            calibrated_msg_count: 0,
            input_token_cost: 0.0,
            output_token_cost: 0.0,
            context_window,
            model: model.to_string(),
            provider: provider.to_string(),
            working_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            next_tool_call_id: 0,
            overhead_tokens: 0,
            rewind_undo: None,
        }
    }

    /// Heuristic token estimate: wide (CJK etc.) chars cost ~0.9 tokens,
    /// narrow chars ~0.25 tokens. Minimum 1 so empty context still counts.
    pub fn estimate_tokens(text: &str) -> u64 {
        let mut wide: u64 = 0;
        let mut narrow: u64 = 0;
        for ch in text.chars() {
            if Self::is_wide_char(ch) {
                wide += 1;
            } else {
                narrow += 1;
            }
        }
        (wide * 9 / 10 + narrow / 4).max(1)
    }

    fn is_wide_char(ch: char) -> bool {
        matches!(ch as u32,
            0x1100..=0x11FF |   // Hangul Jamo
            0x2E80..=0x9FFF |   // CJK radicals/Kangxi/punctuation/kana/Unified+ExtA
            0xA000..=0xA4CF |   // Yi
            0xAC00..=0xD7A3 |   // Hangul Syllables
            0xF900..=0xFAFF |   // CJK Compatibility Ideographs
            0xFF00..=0xFFEF |   // Halfwidth/Fullwidth Forms
            0x20000..=0x3FFFF   // Supplementary Ideographic Plane (Ext B-F)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_is_min_one_and_scales() {
        assert_eq!(Session::estimate_tokens(""), 1);
        assert_eq!(Session::estimate_tokens("abcd"), 1);
        assert_eq!(Session::estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn estimate_counts_wide_chars_heavier() {
        let narrow = Session::estimate_tokens("aaaaaaaa");
        let wide = Session::estimate_tokens("\u{4e2d}\u{4e2d}\u{4e2d}\u{4e2d}");
        assert!(wide > narrow);
    }

    #[test]
    fn runtime_fields_do_not_serialize() {
        let mut s = Session::new("p", "m", 1000, "n");
        s.overhead_tokens = 123;
        s.rewind_undo = Some(RewindUndo {
            messages: Vec::new(),
            total_estimated_tokens: 0,
            calibrated_tokens: 0,
            calibrated_msg_count: 0,
        });
        let v: serde_json::Value = serde_json::to_value(&s).unwrap();
        assert!(v.get("overhead_tokens").is_none());
        assert!(v.get("rewind_undo").is_none());
        assert_eq!(v["name"], "n");
    }
}
