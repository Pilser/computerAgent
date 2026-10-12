//! Prompt assembly: system + memory block + session recap.
//!
//! Pure function (no I/O): the caller supplies the memory block (from
//! `session-store` `inject`) and the recap. An empty/whitespace memory block
//! is omitted so the model never sees an empty `<memory></memory>`.

/// Assemble the final prompt from its three sections.
pub fn assemble(system: &str, memory: Option<&str>, recap: &str) -> String {
    let mut out = String::new();
    out.push_str(system);
    if let Some(block) = memory.filter(|m| !m.trim().is_empty()) {
        out.push_str("\n\n<memory>\n");
        out.push_str(block.trim());
        out.push_str("\n</memory>");
    }
    if !recap.trim().is_empty() {
        out.push_str("\n\n## Session recap\n");
        out.push_str(recap.trim());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_all_three_sections() {
        let p = assemble("SYS", Some("prefers fish"), "did X, next Y");
        assert!(p.contains("SYS"));
        assert!(p.contains("<memory>\nprefers fish\n</memory>"));
        assert!(p.contains("## Session recap\ndid X, next Y"));
        // Order: system, memory, recap.
        let si = p.find("SYS").unwrap();
        let mi = p.find("<memory>").unwrap();
        let ri = p.find("## Session recap").unwrap();
        assert!(si < mi && mi < ri);
    }

    #[test]
    fn none_memory_omits_block() {
        let p = assemble("SYS", None, "recap");
        assert!(!p.contains("<memory>"));
        assert!(p.contains("recap"));
    }

    #[test]
    fn empty_and_whitespace_memory_omitted() {
        for m in ["", "   ", "\n\t "] {
            let p = assemble("SYS", Some(m), "recap");
            assert!(!p.contains("<memory>"), "{m:?}");
        }
    }

    #[test]
    fn empty_recap_omitted() {
        let p = assemble("SYS", Some("mem"), "  ");
        assert!(p.contains("<memory>"));
        assert!(!p.contains("Session recap"));
    }
}
