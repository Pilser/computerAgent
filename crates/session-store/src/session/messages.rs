//! Message mutation primitives. Port of the `Session` message methods in
//! `_vendor/embedded-agent/src/session/mod.rs`. The tool-call summary is a
//! plain `name + args JSON` line (the vendor formats it via `ui::utils`,
//! which is deliberately not ported).

use std::path::Path;

use super::storage;
use super::types::{MessageRole, Session, ToolRecord};

/// Outputs longer than this are spilled to a sidecar file; the message
/// keeps a head/tail excerpt plus the file path.
pub const TOOL_RESULT_SAVE_THRESHOLD: usize = 12_000;
pub const TOOL_RESULT_HEAD_CHARS: usize = 2_000;
pub const TOOL_RESULT_TAIL_CHARS: usize = 8_000;

impl Session {
    pub fn add_message(&mut self, role: MessageRole, content: &str) {
        self.add_message_with_tool(role, content, None);
    }

    fn add_message_with_tool(
        &mut self,
        role: MessageRole,
        content: &str,
        tool: Option<ToolRecord>,
    ) {
        let tokens = Self::estimate_tokens(content);
        self.messages.push(super::types::SessionMessage {
            role,
            content: content.to_string(),
            estimated_tokens: tokens,
            tool,
            turn: None,
        });
        self.total_estimated_tokens = self.total_estimated_tokens.saturating_add(tokens);
        self.updated_at = chrono::Utc::now().to_rfc3339();
        // The conversation moved forward, so a pending rewind restore point
        // no longer lines up — drop it so redo can't splice in a stale tail.
        self.rewind_undo = None;
    }

    /// Records a tool call with its full argument JSON and returns the
    /// stamped id so the caller can link the matching result via
    /// [`add_tool_result`](Self::add_tool_result).
    pub fn add_tool_call(&mut self, name: &str, args: &serde_json::Value) -> u64 {
        let id = self.next_tool_call_id;
        self.next_tool_call_id = self.next_tool_call_id.saturating_add(1);
        let content = format!("{name} {args}");
        self.add_message_with_tool(
            MessageRole::ToolCall,
            &content,
            Some(ToolRecord::Call {
                id,
                name: name.to_string(),
                args: args.clone(),
            }),
        );
        id
    }

    /// Records a tool result; `call_id` must be the id returned by the
    /// [`add_tool_call`](Self::add_tool_call) this result answers.
    pub fn add_tool_result(&mut self, call_id: u64, name: &str, output: &str) -> String {
        let base = storage::data_dir();
        self.add_tool_result_in(&base, call_id, name, output)
    }

    /// [`add_tool_result`](Self::add_tool_result) with an explicit data dir
    /// (tests use a temp dir so they never touch the real data dir).
    pub fn add_tool_result_in(
        &mut self,
        base: &Path,
        call_id: u64,
        name: &str,
        output: &str,
    ) -> String {
        let (content, truncated, full_output_path) =
            Self::tool_result_content(base, &self.id, name, output);
        self.add_message_with_tool(
            MessageRole::ToolResult,
            &content,
            Some(ToolRecord::Result {
                call_id,
                name: name.to_string(),
                truncated,
                full_output_path,
            }),
        );
        content
    }

    fn tool_result_content(
        base: &Path,
        session_id: &str,
        name: &str,
        output: &str,
    ) -> (String, bool, Option<String>) {
        let output_chars = output.chars().count();
        if output_chars <= TOOL_RESULT_SAVE_THRESHOLD {
            return (format!("{name}:\n{output}"), false, None);
        }
        match storage::save_tool_output_in(base, session_id, name, output) {
            Ok(path) => (
                format_truncated_tool_result(name, output, output_chars, &path),
                true,
                Some(path.display().to_string()),
            ),
            Err(err) => (
                format!(
                    "{name}:\n{output}\n\n[failed to save long tool output separately; kept full output in session to avoid data loss: {err}]"
                ),
                false,
                None,
            ),
        }
    }

    /// The true prompt size occupying the context window, normalizing across
    /// providers' differing cache-usage reporting. `anthropic_native` must be
    /// the resolved protocol route: native reports cache tokens separately
    /// (add them), other shapes already fold them into `input_tokens`.
    pub fn real_input_tokens(
        anthropic_native: bool,
        input_tokens: u64,
        cached_input_tokens: u64,
        cache_creation_input_tokens: u64,
    ) -> u64 {
        if anthropic_native {
            input_tokens
                .saturating_add(cached_input_tokens)
                .saturating_add(cache_creation_input_tokens)
        } else {
            input_tokens
        }
    }

    pub fn set_calibration(&mut self, input_tokens: u64, output_tokens: u64) {
        if input_tokens == 0 {
            return;
        }
        self.calibrated_tokens = input_tokens.saturating_add(output_tokens);
        self.calibrated_msg_count = self.messages.len();
    }

    pub fn reset_calibration(&mut self) {
        self.calibrated_tokens = 0;
        self.calibrated_msg_count = 0;
    }

    /// Truncate to `new_len` messages while keeping the calibration anchor
    /// accurate: if removed messages were part of the anchor, subtract their
    /// estimates instead of dropping the whole calibration.
    pub fn truncate_to(&mut self, new_len: usize) {
        if new_len >= self.messages.len() {
            return;
        }
        let cal = self.calibrated_msg_count.min(self.messages.len());
        if self.calibrated_tokens > 0 && new_len < cal {
            let removed: u64 = self.messages[new_len..cal]
                .iter()
                .map(|m| m.estimated_tokens)
                .sum();
            self.calibrated_tokens = self.calibrated_tokens.saturating_sub(removed);
            self.calibrated_msg_count = new_len;
        }
        self.messages.truncate(new_len);
        self.total_estimated_tokens = self.messages.iter().map(|m| m.estimated_tokens).sum();
    }

    /// Rewind to `new_len` messages, capturing a single-step restore point
    /// first so the cut can be undone with [`redo`](Self::redo). Returns the
    /// number of messages removed (0 when nothing was cut).
    pub fn rewind_to(&mut self, new_len: usize) -> usize {
        if new_len >= self.messages.len() {
            return 0;
        }
        let removed = self.messages.len() - new_len;
        self.rewind_undo = Some(super::types::RewindUndo {
            messages: self.messages.clone(),
            total_estimated_tokens: self.total_estimated_tokens,
            calibrated_tokens: self.calibrated_tokens,
            calibrated_msg_count: self.calibrated_msg_count,
        });
        self.truncate_to(new_len);
        removed
    }

    /// Restore the messages removed by the most recent
    /// [`rewind_to`](Self::rewind_to). Returns false when there is nothing
    /// to restore. The restore point is consumed on use.
    pub fn redo(&mut self) -> bool {
        match self.rewind_undo.take() {
            Some(u) => {
                self.messages = u.messages;
                self.total_estimated_tokens = u.total_estimated_tokens;
                self.calibrated_tokens = u.calibrated_tokens;
                self.calibrated_msg_count = u.calibrated_msg_count;
                true
            }
            None => false,
        }
    }

    /// True while no provider usage has been reported yet (or calibration
    /// was reset): the context figure is still an estimate.
    pub fn ctx_is_estimated(&self) -> bool {
        self.calibrated_tokens == 0
    }

    pub fn effective_context_tokens(&self) -> u64 {
        if self.calibrated_tokens == 0 {
            return self
                .overhead_tokens
                .saturating_add(self.total_estimated_tokens);
        }
        let start = self.calibrated_msg_count.min(self.messages.len());
        let delta: u64 = self.messages[start..]
            .iter()
            .map(|m| m.estimated_tokens)
            .sum();
        self.calibrated_tokens.saturating_add(delta)
    }

    pub fn update_context_window(&mut self, cw: u64) {
        self.context_window = cw;
    }
}

fn format_truncated_tool_result(
    name: &str,
    output: &str,
    output_chars: usize,
    path: &Path,
) -> String {
    let head_end = output
        .char_indices()
        .nth(TOOL_RESULT_HEAD_CHARS)
        .map(|(i, _)| i)
        .unwrap_or(output.len());
    let head = &output[..head_end];
    let tail_start_char = output_chars.saturating_sub(TOOL_RESULT_TAIL_CHARS);
    let tail_start = output
        .char_indices()
        .nth(tail_start_char)
        .map(|(i, _)| i)
        .unwrap_or(output.len());
    let tail = &output[tail_start..];
    let omitted = output_chars.saturating_sub(TOOL_RESULT_HEAD_CHARS + TOOL_RESULT_TAIL_CHARS);

    format!(
        "{name}:\n{head}\n\n[tool output truncated: {output_chars} characters; {omitted} omitted]\n[full output saved to: {}; use the read tool on this path to inspect the complete output]\n\n{tail}",
        path.display()
    )
}
