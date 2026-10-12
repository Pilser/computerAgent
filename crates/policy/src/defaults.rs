//! Built-in rules: safe bash allowlist + always-on deny regexes.
//!
//! Port of vendor `default_bash_rules` / `default_deny_regex_rules`
//! (representative subset: the deny shape is load-bearing, not the count).

use crate::decision::Action;

pub fn default_bash_rules() -> Vec<(&'static str, Action)> {
    vec![
        ("ls **", Action::Allow),
        ("cd **", Action::Allow),
        ("pwd", Action::Allow),
        ("echo **", Action::Allow),
        ("cat **", Action::Allow),
        ("git status **", Action::Allow),
        ("git log **", Action::Allow),
        ("git diff **", Action::Allow),
        ("cargo check", Action::Allow),
        ("cargo test", Action::Allow),
        ("mkdir **", Action::Allow),
        ("touch **", Action::Allow),
        ("sudo", Action::Ask),
        ("sudo **", Action::Ask),
        ("rm -rf /**", Action::Deny),
        ("sudo rm -rf /**", Action::Deny),
        ("dd **", Action::Deny),
        ("mkfs **", Action::Deny),
        ("vim **", Action::Deny),
        ("nano **", Action::Deny),
    ]
}

pub fn default_deny_regex_rules() -> Vec<(&'static str, &'static str)> {
    vec![("bash", r"^rm\s+.*\*")]
}
