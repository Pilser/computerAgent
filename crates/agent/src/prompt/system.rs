//! Static system instructions (no user data — never memory).
//!
//! Slim port of vendor `agent/prompt.rs::SYSTEM_PROMPT`, retargeted from
//! "coding assistant" to full-privilege computer assistant. Tool schemas come
//! from the [`ToolRegistry`]; memory arrives separately via
//! [`assemble`](super::assemble).

use crate::tools::ToolRegistry;

/// Static role + rules. Memory and session recap are appended by `assemble`.
pub const ROLE: &str = "\
You are a computer assistant with full access to this machine. Read and write \
files and run commands to accomplish the user's goal. Respond in the user's \
language.

## Conciseness (CRITICAL)
- Keep responses under 4 lines of text (excluding tool calls/code), unless \
the user asks for detail. One-word answers are best.
- Do NOT add preamble/postamble (\"Here is what I'll do...\", \"The answer \
is...\").
- Do NOT explain or summarize your changes unless asked.

## Tools
- Batch independent calls in a single message; chain dependent commands with \
`&&`.
- Read a file before editing it. After editing, re-read the changed area to \
verify.
- If a task needs system intervention (installing packages, modifying system \
config), stop and ask first.
- Ask the user when in doubt — do not guess.";

/// Render one tool schema as a prompt line.
pub fn tool_line(name: &str, description: &str) -> String {
    format!("- **{name}**: {description}")
}

/// Full system prompt = [`ROLE`] + the registry's tool schemas.
pub fn system_prompt(registry: &ToolRegistry) -> String {
    let mut out = String::from(ROLE);
    out.push_str("\n\n## Available tools\n");
    for info in registry.schemas() {
        out.push_str(&tool_line(&info.name, &info.description));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_has_no_user_data_markers() {
        assert!(!ROLE.contains("<memory>"));
        assert!(ROLE.contains("computer assistant"));
    }

    #[test]
    fn prompt_lists_registry_tools() {
        let reg = ToolRegistry::default_coding();
        let p = system_prompt(&reg);
        assert!(p.starts_with(ROLE));
        for name in reg.list() {
            assert!(p.contains(name), "{name}");
        }
    }

    #[test]
    fn empty_registry_still_has_role() {
        let p = system_prompt(&ToolRegistry::new());
        assert!(p.contains("computer assistant"));
    }
}
