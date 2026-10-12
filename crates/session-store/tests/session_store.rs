//! Phase 3 integration tests: message ops, tool linkage,
//! truncate/rewind/redo, storage round-trip, chat history. All filesystem
//! tests use a temp dir under /tmp/opencode and clean up afterwards.

use std::path::PathBuf;

use computer_agent_session_store::session::chat_history::{
    append_entry_in, load_history_in, ChatHistoryEntry,
};
use computer_agent_session_store::session::storage::{
    delete_session_in, find_recent_sessions_in, load_session_in, save_session_in,
};
use computer_agent_session_store::session::types::{MessageRole, Session, ToolRecord};

fn test_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("/tmp/opencode").join(format!(
        "ca-session-store-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &std::path::Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn message_add_updates_token_total() {
    let mut s = Session::new("p", "m", 8000, "t");
    assert_eq!(s.total_estimated_tokens, 0);
    s.add_message(MessageRole::User, "hello world, this is a test message");
    assert_eq!(s.messages.len(), 1);
    assert_eq!(s.messages[0].role, MessageRole::User);
    assert!(s.messages[0].estimated_tokens >= 1);
    assert_eq!(s.total_estimated_tokens, s.messages[0].estimated_tokens);
    assert_eq!(
        Session::estimate_tokens("hello world, this is a test message"),
        s.messages[0].estimated_tokens
    );
}

#[test]
fn tool_call_ids_link_results() {
    let mut s = Session::new("p", "m", 8000, "t");
    let args = serde_json::json!({"path": "a.txt"});
    let id0 = s.add_tool_call("read", &args);
    let id1 = s.add_tool_call("read", &args);
    assert_eq!((id0, id1), (0, 1));

    let dir = test_dir("tool-link");
    s.add_tool_result_in(&dir, id1, "read", "contents");
    s.add_tool_result_in(&dir, id0, "read", "other");
    cleanup(&dir);

    let calls: Vec<u64> = s
        .messages
        .iter()
        .filter_map(|m| match &m.tool {
            Some(ToolRecord::Call { id, .. }) => Some(*id),
            _ => None,
        })
        .collect();
    let results: Vec<u64> = s
        .messages
        .iter()
        .filter_map(|m| match &m.tool {
            Some(ToolRecord::Result { call_id, .. }) => Some(*call_id),
            _ => None,
        })
        .collect();
    assert_eq!(calls, vec![0, 1]);
    // Linkage is by id, not position: out-of-order results still resolve.
    assert_eq!(results, vec![1, 0]);
    // Full args survive on the Call record.
    match &s.messages[0].tool {
        Some(ToolRecord::Call { args: a, .. }) => assert_eq!(*a, args),
        other => panic!("expected Call record, got {other:?}"),
    }
}

#[test]
fn large_tool_result_spills_to_sidecar() {
    let dir = test_dir("tool-spill");
    let mut s = Session::new("p", "m", 8000, "t");
    let big = "x".repeat(20_000);
    let content = s.add_tool_result_in(&dir, 0, "bash", &big);
    assert!(content.contains("[tool output truncated:"));
    match &s.messages[0].tool {
        Some(ToolRecord::Result {
            truncated,
            full_output_path,
            ..
        }) => {
            assert!(truncated);
            let p = full_output_path.as_ref().unwrap();
            assert_eq!(std::fs::read_to_string(p).unwrap(), big);
        }
        other => panic!("expected Result record, got {other:?}"),
    }
    cleanup(&dir);
}

#[test]
fn truncate_rewind_redo() {
    let mut s = Session::new("p", "m", 8000, "t");
    for i in 0..5 {
        s.add_message(
            MessageRole::User,
            &format!("message {i} with some padding text"),
        );
    }
    s.set_calibration(1000, 100);
    assert!(!s.ctx_is_estimated());

    let removed = s.rewind_to(2);
    assert_eq!(removed, 3);
    assert_eq!(s.messages.len(), 2);
    // Anchor covered removed messages: calibration shrinks, stays anchored.
    assert!(s.calibrated_tokens < 1100);

    assert!(s.redo());
    assert_eq!(s.messages.len(), 5);
    assert_eq!(s.calibrated_tokens, 1100);
    assert!(!s.redo());

    // New messages invalidate the restore point.
    s.rewind_to(1);
    s.add_message(MessageRole::User, "brand new turn");
    assert!(!s.redo());

    // truncate_to past-the-end is a no-op.
    let len = s.messages.len();
    s.truncate_to(len + 10);
    assert_eq!(s.messages.len(), len);
}

#[test]
fn storage_round_trip_via_temp_dir() {
    let dir = test_dir("roundtrip");
    let mut s = Session::new("prov", "model-x", 16_000, "work");
    s.add_message(MessageRole::User, "remember this");
    let id = s.add_tool_call("bash", &serde_json::json!({"cmd": "ls"}));
    s.add_tool_result_in(&dir, id, "bash", "a b c");
    s.compress("old summary".to_string(), 1, 50);

    save_session_in(&dir, &s).unwrap();
    let loaded = load_session_in(&dir, &s.id).unwrap().expect("session file");
    assert_eq!(loaded.id, s.id);
    assert_eq!(loaded.name, "work");
    assert_eq!(loaded.model, "model-x");
    assert_eq!(loaded.messages.len(), s.messages.len());
    assert_eq!(loaded.compactions.len(), 1);
    assert_eq!(loaded.next_tool_call_id, 1);
    assert_eq!(loaded.total_estimated_tokens, s.total_estimated_tokens);

    let recent = find_recent_sessions_in(&dir, 10).unwrap();
    assert_eq!(recent.len(), 1);

    assert!(load_session_in(&dir, "missing-id").unwrap().is_none());
    delete_session_in(&dir, &s.id).unwrap();
    assert!(load_session_in(&dir, &s.id).unwrap().is_none());
    cleanup(&dir);
}

#[test]
fn chat_history_jsonl_round_trip() {
    let dir = test_dir("history");
    assert!(load_history_in(&dir).unwrap().is_empty());
    append_entry_in(&dir, &ChatHistoryEntry::now("first")).unwrap();
    append_entry_in(&dir, &ChatHistoryEntry::now("second")).unwrap();
    let loaded = load_history_in(&dir).unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].content, "first");
    assert_eq!(loaded[1].content, "second");

    // One corrupt line must not lose the good entries.
    std::fs::write(dir.join("chat_history.jsonl"), "{bad json\n").unwrap();
    append_entry_in(&dir, &ChatHistoryEntry::now("third")).unwrap();
    let loaded = load_history_in(&dir).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].content, "third");
    cleanup(&dir);
}
