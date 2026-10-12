//! Phase 4 integration tests: memory write dedup, overwrite backup, edit
//! uniqueness errors, note traversal rejection, search ranking, inject cap +
//! empty-store None. All filesystem tests use a temp dir under /tmp/opencode
//! and clean up afterwards.

use std::path::PathBuf;

use computer_agent_session_store::memory::{Mem, WriteMode, WriteTarget};

fn test_mem(name: &str) -> (Mem, PathBuf) {
    let dir =
        PathBuf::from("/tmp/opencode").join(format!("ca-memory-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mem = Mem::open_in(dir.clone(), "proj", "2026-10-12");
    (mem, dir)
}

fn cleanup(dir: &std::path::Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn long_term_append_dedups() {
    let (mem, dir) = test_mem("dedup");
    mem.write(
        WriteTarget::LongTerm,
        "fact alpha\nfact beta",
        WriteMode::Append,
        None,
    )
    .unwrap();
    // Exact + whitespace-variant duplicates are skipped; new lines are kept.
    let msg = mem
        .write(
            WriteTarget::LongTerm,
            "  fact alpha  \nfact beta\nfact gamma",
            WriteMode::Append,
            None,
        )
        .unwrap();
    assert!(msg.contains("skipped 2 duplicate"), "msg: {msg}");
    let content = std::fs::read_to_string(mem.memory_md()).unwrap();
    assert_eq!(content.matches("fact alpha").count(), 1);
    assert_eq!(content.matches("fact beta").count(), 1);
    assert!(content.contains("fact gamma"));

    // A batch of pure duplicates writes nothing and leaves the file alone.
    let before = content.clone();
    let msg = mem
        .write(WriteTarget::LongTerm, "fact alpha", WriteMode::Append, None)
        .unwrap();
    assert!(msg.starts_with("Nothing written"), "msg: {msg}");
    assert_eq!(std::fs::read_to_string(mem.memory_md()).unwrap(), before);

    // Dedup is long-term only: scratchpad keeps byte-identical duplicates.
    mem.write(
        WriteTarget::Scratchpad,
        "- [ ] same",
        WriteMode::Append,
        None,
    )
    .unwrap();
    mem.write(
        WriteTarget::Scratchpad,
        "- [ ] same",
        WriteMode::Append,
        None,
    )
    .unwrap();
    let scratch = std::fs::read_to_string(mem.scratchpad()).unwrap();
    assert_eq!(scratch.matches("- [ ] same").count(), 2);
    cleanup(&dir);
}

#[test]
fn overwrite_backs_up_curated_files() {
    let (mem, dir) = test_mem("backup");
    mem.write(
        WriteTarget::LongTerm,
        "version one",
        WriteMode::Overwrite,
        None,
    )
    .unwrap();
    // First overwrite has nothing to back up: no .bak yet.
    assert!(!mem.memory_md().with_extension("bak").exists());
    mem.write(
        WriteTarget::LongTerm,
        "version two",
        WriteMode::Overwrite,
        None,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(mem.memory_md().with_extension("bak")).unwrap(),
        "version one"
    );
    assert_eq!(
        std::fs::read_to_string(mem.memory_md()).unwrap(),
        "version two"
    );
    cleanup(&dir);
}

#[test]
fn edit_requires_unique_match() {
    let (mem, dir) = test_mem("edit");
    mem.write(
        WriteTarget::LongTerm,
        "dup line\ndup line\nother",
        WriteMode::Overwrite,
        None,
    )
    .unwrap();

    let err = mem
        .edit(WriteTarget::LongTerm, None, Some("dup line"), "x")
        .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(err.to_string().contains("2 time(s)"), "{err}");

    let err = mem
        .edit(WriteTarget::LongTerm, None, Some("missing"), "x")
        .unwrap_err();
    assert!(err.to_string().contains("0 time(s)"), "{err}");

    // File untouched by failed edits.
    assert!(std::fs::read_to_string(mem.memory_md())
        .unwrap()
        .contains("dup line"));

    // Unique match succeeds.
    mem.edit(WriteTarget::LongTerm, None, Some("other"), "replaced")
        .unwrap();
    let content = std::fs::read_to_string(mem.memory_md()).unwrap();
    assert!(content.contains("replaced"));
    assert!(!content.contains("other"));

    // Whole-note delete works; non-note whole delete is rejected.
    mem.write(WriteTarget::Note, "scratch", WriteMode::Append, Some("tmp"))
        .unwrap();
    let note = mem.note_path("tmp").unwrap();
    assert!(note.exists());
    mem.edit(WriteTarget::Note, Some("tmp"), None, "").unwrap();
    assert!(!note.exists());
    let err = mem.edit(WriteTarget::LongTerm, None, None, "").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    cleanup(&dir);
}

#[test]
fn note_names_cannot_traverse() {
    let (mem, dir) = test_mem("traverse");
    assert_eq!(mem.note_path("../../.."), None);
    for bad in ["../evil", "a/b", "..", ".hidden", "a.md.bak"] {
        assert_eq!(mem.note_path(bad), None, "name: {bad}");
        let err = mem
            .write(WriteTarget::Note, "x", WriteMode::Append, Some(bad))
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
    // Nothing escaped the notes dir.
    assert!(!dir.join("evil").exists());
    cleanup(&dir);
}

#[test]
fn search_ranks_memory_first_then_term_coverage() {
    let (mem, dir) = test_mem("search");
    mem.write(
        WriteTarget::LongTerm,
        "the alpha project uses beta tooling",
        WriteMode::Overwrite,
        None,
    )
    .unwrap();
    mem.write(
        WriteTarget::Note,
        "just alpha here",
        WriteMode::Append,
        Some("scoped"),
    )
    .unwrap();

    let results = mem.search("alpha beta");
    assert!(results.hits.len() >= 2);
    // MEMORY.md (both terms) outranks the note (one term).
    assert!(results.hits[0].is_memory_md);
    assert_eq!(results.hits[0].matched_terms, vec!["alpha", "beta"]);
    assert_eq!(results.hits[1].matched_terms, vec!["alpha"]);

    let rendered = results.render(32 * 1024);
    assert!(rendered.contains("alpha(2)"));
    assert!(rendered.contains("beta(1)"));

    // Tight budget drops the least-relevant file first and says so (400 is
    // sized so the omission marker survives the final truncation backstop).
    let tiny = results.render(400);
    assert!(tiny.len() <= 400 + 64, "len: {}", tiny.len());
    assert!(tiny.contains("more file"), "{tiny}");

    // Empty query yields no hits, not an error.
    assert!(mem.search("   ").hits.is_empty());
    cleanup(&dir);
}

#[test]
fn inject_empty_is_none_and_large_is_capped() {
    let (mem, dir) = test_mem("inject-empty");
    assert_eq!(mem.context_block(), None);

    // Scratchpad with no open items does not inject either.
    mem.write(
        WriteTarget::Scratchpad,
        "- [x] done item",
        WriteMode::Append,
        None,
    )
    .unwrap();
    assert_eq!(mem.context_block(), None);

    // Open items + daily log inject with the safety wrapper.
    mem.write(
        WriteTarget::Scratchpad,
        "- [ ] open task",
        WriteMode::Append,
        None,
    )
    .unwrap();
    mem.append_daily("sync", "did things").unwrap();
    let block = mem.context_block().expect("content present");
    assert!(block.starts_with("<memory"));
    assert!(block.contains("- [ ] open task"));
    assert!(!block.contains("- [x] done item"));

    // A 40 KiB MEMORY.md truncates to the 32 KiB budget (plus wrapper).
    let (big, big_dir) = test_mem("inject-cap");
    big.write(
        WriteTarget::LongTerm,
        &"fact line\n".repeat(4000),
        WriteMode::Overwrite,
        None,
    )
    .unwrap();
    let block = big.context_block().expect("content present");
    assert!(block.len() <= 32 * 1024 + 256, "len: {}", block.len());
    assert!(block.contains("section truncated"));
    cleanup(&dir);
    cleanup(&big_dir);
}

#[test]
fn list_and_read_round_trip() {
    let (mem, dir) = test_mem("list");
    assert!(mem.list().is_empty());
    assert_eq!(Mem::read_capped(&mem.memory_md()), "(empty)".to_string());
    mem.write(
        WriteTarget::LongTerm,
        "remember me",
        WriteMode::Append,
        None,
    )
    .unwrap();
    mem.write(WriteTarget::Note, "ref", WriteMode::Append, Some("guide"))
        .unwrap();
    let listed = mem.list().join("\n");
    assert!(listed.contains("MEMORY.md"));
    assert!(listed.contains("guide.md"));
    assert!(Mem::read_capped(&mem.memory_md()).contains("remember me"));
    // Backups never surface in list or search.
    mem.write(WriteTarget::LongTerm, "v2", WriteMode::Overwrite, None)
        .unwrap();
    assert!(!mem.list().join("\n").contains(".bak"));
    assert!(Mem::read_capped(&mem.memory_md()).contains("v2"));
    cleanup(&dir);
}
