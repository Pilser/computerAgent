//! `bash` tool: run a shell command through the sys-tools sandbox.
//!
//! Port of vendor `agent/tools/bash.rs`, trimmed: no `rtk` rewrite, no
//! mask/network failure hints (sys-tools exposes no hint helpers in v1),
//! policy gating instead of `PermCheck` + ask channel. The sandbox defaults
//! to OFF (`Sandbox::default()` runs bare); a required-but-missing backend
//! is refused before policy is consulted, as in the vendor port source.

use serde::{Deserialize, Serialize};

use crate::error::ToolError;
use crate::tools::{check_policy, SharedPolicy, Tool};

#[derive(Debug, Deserialize, Serialize)]
pub struct BashArgs {
    pub command: String,
    pub timeout: Option<u64>,
    #[serde(default)]
    pub note: Option<String>,
}

pub struct BashTool {
    pub policy: Option<SharedPolicy>,
    pub sandbox: computer_agent_sys_tools::sandbox::Sandbox,
    /// `None` = no truncation. `Some(n)` = head-only output after `n` lines.
    pub max_output_lines: Option<u64>,
}

impl BashTool {
    pub fn new(
        policy: Option<SharedPolicy>,
        sandbox: computer_agent_sys_tools::sandbox::Sandbox,
        max_output_lines: Option<u64>,
    ) -> Self {
        Self {
            policy,
            sandbox,
            max_output_lines,
        }
    }
}

impl Tool for BashTool {
    const NAME: &'static str = "bash";
    type Args = BashArgs;

    fn description(&self) -> String {
        "Execute a bash command in the current working directory. Returns stdout and stderr."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Bash command to execute" },
                "timeout": { "type": "integer", "description": "Timeout in milliseconds (optional)" },
                "note": { "type": "string", "description": "Short human note shown in the UI next to this action (optional)" }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, args: BashArgs) -> Result<String, ToolError> {
        // Refuse before asking: a required sandbox with no backend fails every
        // command, so a policy check first would be wasted (vendor order).
        if let Some(reason) = self.sandbox.refusal_reason() {
            return Err(ToolError::Msg(reason));
        }
        // Policy per sub-command, checked against the original text (port).
        for cmd in split_bash_commands(&args.command) {
            check_policy(&self.policy, Self::NAME, &cmd)?;
        }

        let output = if let Some(ms) = args.timeout {
            match tokio::time::timeout(
                std::time::Duration::from_millis(ms),
                self.sandbox.output_command(&args.command),
            )
            .await
            {
                // NOTE: vendor also kills the active child handle; sys-tools
                // v1 sets `kill_on_drop(true)`, so dropping on timeout reaps it.
                Ok(output) => output?,
                Err(_) => return Err(ToolError::Msg("Command timed out".to_string())),
            }
        } else {
            self.sandbox.output_command(&args.command).await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&stderr);
        }
        if exit_code != 0 {
            result.push_str(&format!("\nExit code: {exit_code}"));
        }
        if let Some(cap) = self.max_output_lines {
            result = truncate_lines(&result, cap as usize);
        }
        Ok(result)
    }
}

/// Split on `;`, newline, `&&`, `||` outside quotes (port of the vendor
/// helper, minus the `>`/`>>` redirect splitting which needs no per-target
/// policy check in v1).
pub(crate) fn split_bash_commands(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut single = false;
    let mut double = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            current.push(ch);
            if let Some(next) = chars.next() {
                current.push(next);
            }
        } else if ch == '\'' && !double {
            single = !single;
            current.push(ch);
        } else if ch == '"' && !single {
            double = !double;
            current.push(ch);
        } else if (ch == ';' || ch == '\n') && !single && !double {
            push_cmd(&mut result, &mut current);
        } else if (ch == '&' || ch == '|') && !single && !double {
            if chars.peek() == Some(&ch) {
                chars.next();
                push_cmd(&mut result, &mut current);
            } else {
                current.push(ch);
            }
        } else {
            current.push(ch);
        }
    }
    push_cmd(&mut result, &mut current);
    result
}

fn push_cmd(result: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }
    current.clear();
}

fn truncate_lines(result: &str, cap: usize) -> String {
    let lines: Vec<&str> = result.lines().collect();
    if lines.len() <= cap {
        return result.to_string();
    }
    format!(
        "{}\n\n[truncated after {cap} lines — {} more lines elided; re-run with a narrower invocation or pipe through `tail`/`grep` to see trailing output]",
        lines[..cap].join("\n"),
        lines.len() - cap,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::open_policy;

    fn tool() -> BashTool {
        BashTool::new(
            Some(open_policy()),
            computer_agent_sys_tools::sandbox::Sandbox::default(),
            None,
        )
    }

    #[test]
    fn split_respects_quotes() {
        assert_eq!(
            split_bash_commands("echo 'a;b' && echo ok"),
            vec!["echo 'a;b'", "echo ok"]
        );
        assert_eq!(
            split_bash_commands("true;  ;\nfalse"),
            vec!["true", "false"]
        );
        assert_eq!(split_bash_commands("a | b"), vec!["a | b"]);
    }

    #[tokio::test]
    async fn echo_returns_stdout() {
        let out = tool()
            .call(BashArgs {
                command: "echo hello-bash".into(),
                timeout: None,
                note: None,
            })
            .await
            .unwrap();
        assert!(out.contains("hello-bash"), "{out}");
    }

    #[tokio::test]
    async fn timeout_blocks_long_command() {
        let err = tool()
            .call(BashArgs {
                command: "sleep 30".into(),
                timeout: Some(200),
                note: None,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("timed out"), "{err}");
    }

    #[tokio::test]
    async fn nonzero_exit_reports_code() {
        let out = tool()
            .call(BashArgs {
                command: "exit 3".into(),
                timeout: None,
                note: None,
            })
            .await
            .unwrap();
        assert!(out.contains("Exit code: 3"), "{out}");
    }

    #[tokio::test]
    async fn default_deny_rule_blocks_before_exec() {
        // Vendor default bash deny rules (e.g. `rm -rf`) hold here too: the
        // policy verdict must arrive without running anything.
        let out = tool()
            .call(BashArgs {
                command: "rm -rf /data".into(),
                timeout: None,
                note: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(out, ToolError::Denied(_)), "{out}");
    }

    #[test]
    fn truncation_caps_long_output() {
        let s = (1..=10)
            .map(|i| format!("l{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = truncate_lines(&s, 4);
        assert!(out.contains("l4"));
        assert!(!out.contains("l5\n"));
        assert!(out.contains("truncated after 4 lines"));
    }
}
