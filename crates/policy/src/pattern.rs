//! Glob + regex patterns (port of vendor `permission/pattern.rs`).
//!
//! Glob `*` / `**` / `?` compile to regex; raw regex passes through.
//! A broken pattern fails safe to match-all so a bad deny still denies.

use std::sync::OnceLock;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct Pattern {
    regex: OnceLock<Regex>,
    pub original: String,
    is_regex: bool,
}

impl Pattern {
    pub fn glob(pattern: &str) -> Self {
        Self {
            regex: OnceLock::new(),
            original: pattern.to_string(),
            is_regex: false,
        }
    }

    pub fn new_regex(pattern: &str) -> Self {
        Self {
            regex: OnceLock::new(),
            original: pattern.to_string(),
            is_regex: true,
        }
    }

    pub fn matches(&self, input: &str) -> bool {
        let re = self.regex.get_or_init(|| {
            let expanded = expand_tilde(&self.original);
            let src = if self.is_regex {
                expanded
            } else {
                glob_to_regex(&expanded)
            };
            Regex::new(&src).unwrap_or_else(|_| Regex::new("(?s).*").unwrap())
        });
        re.is_match(input)
    }
}

fn glob_to_regex(pattern: &str) -> String {
    let mut re = String::with_capacity(pattern.len() * 2 + 2);
    re.push('^');
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    chars.next();
                    re.push_str("(?:.*/)?");
                } else {
                    re.push_str(".*");
                }
            }
            '*' => re.push_str("[^/]*"),
            '?' => re.push('.'),
            '.' | '\\' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '^' | '$' | '|' => {
                re.push('\\');
                re.push(c);
            }
            _ => re.push(c),
        }
    }
    re.push('$');
    re
}

pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matching() {
        assert!(Pattern::glob("*.log").matches("app.log"));
        assert!(!Pattern::glob("*.log").matches("app.log/bak"));
        assert!(Pattern::glob("src/**").matches("src/a/b.rs"));
        assert!(Pattern::glob("ls *").matches("ls foo"));
        assert!(!Pattern::glob("ls *").matches("rm foo"));
    }

    #[test]
    fn regex_matching() {
        assert!(Pattern::new_regex(r"^rm\s+.*\*$").matches("rm -rf foo*"));
        assert!(!Pattern::new_regex(r"^rm\s+.*\*$").matches("rm -rf foo"));
    }

    #[test]
    fn broken_pattern_fails_safe_to_match_all() {
        assert!(Pattern::new_regex("([unclosed").matches("anything at all"));
    }
}
