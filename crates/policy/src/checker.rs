//! Rule checker (port of vendor `permission/checker.rs`).
//!
//! Order per call: `todo_write` fast-path → deny rules (security baseline,
//! never bypassed) → session allowlist → mode-resolved rule match →
//! doom-loop guard. Trimmed from vendor: no `mcp`/`hooks` branches (no MCP
//! crate in v1) and a representative default bash-rule subset.

use std::collections::HashMap;

use crate::decision::{Action, Decision, Policy, SecurityMode};
use crate::doom_loop::DoomLoop;
use crate::pattern::{expand_tilde, Pattern};
use crate::store::SessionAllowlist;

pub struct PermissionChecker {
    rules: HashMap<String, Vec<(Pattern, Action)>>,
    default_action: Action,
    doom_action: Action,
    mode: SecurityMode,
    user_mode: SecurityMode,
    allowlist: SessionAllowlist,
    doom: DoomLoop,
    working_dir: String,
}

impl PermissionChecker {
    pub fn new(mode: SecurityMode) -> Self {
        let working_dir = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let mut c = Self {
            rules: HashMap::new(),
            default_action: Action::Allow,
            doom_action: Action::Ask,
            mode,
            user_mode: mode,
            allowlist: SessionAllowlist::new(),
            doom: DoomLoop::default(),
            working_dir,
        };
        c.with_default_bash_rules();
        c
    }

    pub fn rule(&mut self, tool: &str, pattern: &str, is_regex: bool, action: Action) {
        let pat = if is_regex {
            Pattern::new_regex(pattern)
        } else {
            Pattern::glob(pattern)
        };
        self.rules
            .entry(tool.to_string())
            .or_default()
            .push((pat, action));
    }

    pub fn ask(&mut self, tool: &str, pattern: &str) {
        self.rule(tool, pattern, false, Action::Ask);
    }

    pub fn deny(&mut self, tool: &str, pattern: &str) {
        self.rule(tool, pattern, false, Action::Deny);
    }

    pub fn deny_regex(&mut self, tool: &str, regex: &str) {
        self.rule(tool, regex, true, Action::Deny);
    }

    pub fn with_default(&mut self, action: Action) {
        self.default_action = action;
    }

    pub fn with_doom_action(&mut self, action: Action) {
        self.doom_action = action;
    }

    pub fn set_mode(&mut self, mode: SecurityMode) {
        self.mode = mode;
        self.user_mode = mode;
    }

    pub fn set_prompt_mode(&mut self, mode: SecurityMode) {
        self.mode = mode;
    }

    pub fn restore_user_mode(&mut self) {
        self.mode = self.user_mode;
    }

    pub fn mode(&self) -> SecurityMode {
        self.mode
    }

    /// Representative subset of vendor `default_bash_rules` (~80 entries).
    /// See [`defaults`](crate::defaults) for the table.
    fn with_default_bash_rules(&mut self) {
        for (pat, action) in crate::defaults::default_bash_rules() {
            self.rules
                .entry("bash".to_string())
                .or_default()
                .push((Pattern::glob(pat), action));
        }
        for (tool, regex) in crate::defaults::default_deny_regex_rules() {
            self.deny_regex(tool, regex);
        }
    }

    pub fn check(&mut self, tool: &str, input: &str) -> Decision {
        if tool == "todo_write" {
            return Decision::Allow;
        }
        if self.matches_deny(tool, &[input]) {
            return Decision::Deny("Blocked by deny rule".into());
        }
        if self.allowlist.is_allowed(tool, input) {
            return Decision::Allow;
        }
        let matched = self.collect(tool, &[input]);
        let action = self.resolve(tool, &matched, None);
        self.finish(tool, input, action)
    }

    pub fn check_path(&mut self, tool: &str, path: &str) -> Decision {
        if tool == "todo_write" {
            return Decision::Allow;
        }
        let expanded = expand_tilde(path);
        let abs = absolutize(&expanded, &self.working_dir);
        if self.matches_deny(tool, &[&abs, &expanded]) {
            return Decision::Deny("Blocked by deny rule".into());
        }
        if self.allowlist.is_allowed(tool, &expanded) || self.allowlist.is_allowed(tool, &abs) {
            return Decision::Allow;
        }
        let matched = self.collect(tool, &[&abs, &expanded]);
        let action = self.resolve(tool, &matched, Some(&abs));
        self.finish(tool, &expanded, action)
    }

    fn collect(&self, tool: &str, inputs: &[&str]) -> Vec<Action> {
        let mut out = Vec::new();
        if let Some(rules) = self.rules.get(tool) {
            for (pat, action) in rules {
                if inputs.iter().any(|i| pat.matches(i)) {
                    out.push(*action);
                }
            }
        }
        out
    }

    fn matches_deny(&self, tool: &str, inputs: &[&str]) -> bool {
        self.rules.get(tool).is_some_and(|rules| {
            rules
                .iter()
                .any(|(p, a)| *a == Action::Deny && inputs.iter().any(|i| p.matches(i)))
        })
    }

    /// Last matching rule wins; on no match the mode decides. Merged port
    /// of vendor `resolve_check_action` / `resolve_path_action`.
    fn resolve(&self, tool: &str, matched: &[Action], abs: Option<&str>) -> Action {
        let base = matched.last().copied();
        match self.mode {
            SecurityMode::Restrictive => base.unwrap_or(Action::Ask),
            SecurityMode::ReadOnly => base.unwrap_or_else(|| {
                if is_read_tool(tool) {
                    Action::Allow
                } else {
                    Action::Deny
                }
            }),
            SecurityMode::PlanWrite => base.unwrap_or_else(|| {
                let plan = abs.is_some_and(is_plan_file);
                if is_read_tool(tool) || ((tool == "write" || tool == "edit") && plan) {
                    Action::Allow
                } else {
                    Action::Deny
                }
            }),
            SecurityMode::Guarded => base.unwrap_or_else(|| {
                if is_read_tool(tool) {
                    Action::Allow
                } else {
                    Action::Ask
                }
            }),
            SecurityMode::Standard => base.unwrap_or(self.default_action),
            SecurityMode::Yolo => match base {
                Some(Action::Deny) => Action::Deny,
                Some(other) => other,
                None => Action::Allow,
            },
        }
    }

    /// Doom-loop guard: repeats of an `Allow` stay allowed (vendor coaches);
    /// repeats of an `Ask` escalate per `doom_action`. Denies never feed it.
    fn finish(&mut self, tool: &str, key: &str, action: Action) -> Decision {
        if action != Action::Deny {
            let n = self.doom.record(tool, key);
            if self.doom.is_loop() {
                match (action, self.doom_action) {
                    (Action::Allow, _) => return Decision::Allow,
                    (_, Action::Deny) => {
                        return Decision::Deny(format!("Doom loop: {tool} repeated {n}x"));
                    }
                    _ => return Decision::Ask,
                }
            }
        }
        match action {
            Action::Allow => Decision::Allow,
            Action::Ask => Decision::Ask,
            Action::Deny => Decision::Deny("Blocked by permission rules".into()),
        }
    }
}

impl Policy for PermissionChecker {
    fn check(&mut self, tool: &str, input: &str) -> Decision {
        PermissionChecker::check(self, tool, input)
    }

    fn check_path(&mut self, tool: &str, path: &str) -> Decision {
        PermissionChecker::check_path(self, tool, path)
    }

    fn allow_session(&mut self, tool: &str, pattern: &str) {
        self.allowlist.add(tool, pattern);
    }
}

fn is_read_tool(tool: &str) -> bool {
    matches!(
        tool,
        "read" | "grep" | "find_files" | "list_dir" | "subagent"
    )
}

fn is_plan_file(path: &str) -> bool {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("PLAN") && n.ends_with(".md"))
}

fn absolutize(path: &str, working_dir: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        path.to_string()
    } else {
        std::path::Path::new(working_dir)
            .join(p)
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_allow_ask_deny() {
        let mut c = PermissionChecker::new(SecurityMode::Guarded);
        c.rule("read", "/tmp/**", false, Action::Allow);
        c.ask("bash", "deploy *");
        assert_eq!(c.check_path("read", "/tmp/a.txt"), Decision::Allow);
        assert_eq!(c.check("bash", "deploy prod"), Decision::Ask);
        assert_eq!(
            c.check("bash", "rm -rf /data"),
            Decision::Deny("Blocked by deny rule".into())
        );
    }

    #[test]
    fn allowlist_add_permits() {
        let mut c = PermissionChecker::new(SecurityMode::Restrictive);
        assert_eq!(c.check("bash", "custom-tool --flag"), Decision::Ask);
        c.allow_session("bash", "custom-tool *");
        assert_eq!(c.check("bash", "custom-tool --flag"), Decision::Allow);
        // Allowlist never bypasses a deny rule.
        c.allow_session("bash", "rm -rf *");
        assert!(matches!(c.check("bash", "rm -rf /data"), Decision::Deny(_)));
    }

    #[test]
    fn doom_loop_escalates_after_repeats() {
        let mut c = PermissionChecker::new(SecurityMode::Guarded);
        c.with_doom_action(Action::Deny);
        c.ask("bash", "stuck *");
        assert_eq!(c.check("bash", "stuck cmd"), Decision::Ask);
        assert_eq!(c.check("bash", "stuck cmd"), Decision::Ask);
        assert!(matches!(c.check("bash", "stuck cmd"), Decision::Deny(_)));
    }
}
