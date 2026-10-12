//! End-to-end turn tests against a temp session store (`/tmp/opencode`).
//!
//! Phase 8 contract: a policy-denied run blocks without persisting, a stub
//! run persists user + assistant messages, [`Engine::dispatch`] routes
//! `AgentRun`/`AgentCancel`, and the stream hub fans out `Text` + `Done`.

use std::path::PathBuf;

use command_api::{AgentCancelArgs, AgentRunArgs, Command, CommandResult};
use computer_agent_engine::{Engine, EngineError, TurnEvent, AGENT_RUN_TOOL};
use computer_agent_policy::{PermissionChecker, SecurityMode};
use computer_agent_session_store::session::{MessageRole, Session};

fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(format!("/tmp/opencode/engine-tests/{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[tokio::test]
async fn policy_denied_run_is_blocked() {
    let dir = temp_dir("denied");
    let mut checker = PermissionChecker::new(SecurityMode::Standard);
    checker.deny(AGENT_RUN_TOOL, "**");
    let engine = Engine::with_store_dir(dir.clone()).with_policy(checker);

    let err = engine
        .handle_agent_run("hello", None)
        .await
        .expect_err("deny rule must block the run");
    assert!(matches!(err, EngineError::Denied(_)));

    assert!(
        !dir.join("sessions").exists(),
        "denied run must not persist"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn stub_run_persists_user_and_assistant_messages() {
    let dir = temp_dir("persist");
    let engine = Engine::with_store_dir(dir.clone());
    let mut rx = engine.events().subscribe(16);

    let result = engine
        .handle_agent_run("do the thing", None)
        .await
        .expect("stub run");
    let run_id = match result {
        CommandResult::AgentAccepted { run_id } => run_id,
        other => panic!("expected AgentAccepted, got {other:?}"),
    };
    assert!(!run_id.is_empty());

    assert!(matches!(rx.recv().await, Some(TurnEvent::Text(_))));
    assert!(matches!(rx.recv().await, Some(TurnEvent::Done { .. })));

    let files: Vec<_> = std::fs::read_dir(dir.join("sessions"))
        .expect("sessions dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .collect();
    assert_eq!(files.len(), 1, "one session file, got {files:?}");
    let session: Session =
        serde_json::from_str(&std::fs::read_to_string(files[0].path()).expect("read"))
            .expect("parse");
    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[0].role, MessageRole::User);
    assert_eq!(session.messages[0].content, "do the thing");
    assert_eq!(session.messages[1].role, MessageRole::Assistant);
    assert!(!session.messages[1].content.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn dispatch_routes_agent_run_and_cancel() {
    let dir = temp_dir("dispatch");
    let engine = Engine::with_store_dir(dir.clone());

    let result = engine
        .dispatch(Command::AgentRun(AgentRunArgs {
            prompt: "hi".to_string(),
            session_id: None,
        }))
        .await
        .expect("dispatch run");
    assert!(matches!(result, CommandResult::AgentAccepted { .. }));

    let result = engine
        .dispatch(Command::AgentCancel(AgentCancelArgs {
            run_id: "unknown".to_string(),
        }))
        .await
        .expect("dispatch cancel");
    assert!(matches!(result, CommandResult::AgentCancelled));

    let err = engine
        .dispatch(Command::SystemInfo)
        .await
        .expect_err("SystemInfo is out of scope");
    assert!(matches!(err, EngineError::Unsupported(_)));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn empty_prompt_is_invalid() {
    let dir = temp_dir("empty");
    let engine = Engine::with_store_dir(dir.clone());
    let err = engine
        .handle_agent_run("   ", None)
        .await
        .expect_err("empty prompt");
    assert!(matches!(err, EngineError::InvalidArgs(_)));
    let _ = std::fs::remove_dir_all(&dir);
}
