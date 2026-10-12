//! JSON wire shape shared with the future TypeScript side.
//!
//! Helpers only — no I/O, no window. `{"type":"AgentRun","data":{...}}`
//! deserializes into the same [`Command`](command-api::Command) the Rust
//! shells dispatch, so `api.ts` (when it lands) just `invoke`s these
//! payloads. Example:
//!
//! ```json
//! {"type": "SystemInfo"}
//! {"type": "AgentRun", "data": {"prompt": "hi", "session_id": null}}
//! ```

use command_api::Command;

/// Human-readable note pointing TS authors at the serde contract.
pub const COMMAND_SCHEMA_NOTE: &str = "wire format = command-api::Command serde \
    (tag = \"type\", content = \"data\"); see module docs for examples";

/// Serialize a [`Command`] to the JSON the web view will send.
#[must_use]
pub fn command_to_json(cmd: &Command) -> String {
    serde_json::to_string(cmd).expect("Command is serializable")
}

/// Parse web-view JSON back into a [`Command`].
///
/// # Errors
///
/// [`serde_json::Error`] on malformed payloads.
pub fn command_from_json(json: &str) -> Result<Command, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use command_api::AgentRunArgs;

    #[test]
    fn system_info_json_shape() {
        assert_eq!(
            command_to_json(&Command::SystemInfo),
            r#"{"type":"SystemInfo"}"#
        );
        assert_eq!(
            command_from_json(r#"{"type":"SystemInfo"}"#).expect("parse"),
            Command::SystemInfo
        );
    }

    #[test]
    fn agent_run_json_round_trip() {
        let cmd = Command::AgentRun(AgentRunArgs {
            prompt: "hi".to_string(),
            session_id: None,
        });
        let back = command_from_json(&command_to_json(&cmd)).expect("round trip");
        assert_eq!(back, cmd);
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(command_from_json(r#"{"type":"Nope"}"#).is_err());
    }
}
