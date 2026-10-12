//! Tauri application entry: real `tauri::Builder` wiring.
//!
//! Compiled ONLY with `--features tauri-app` (needs system webkit, see
//! `docs/ARCHITECTURE.md`). Default local builds skip this module, so
//! `./scripts/ram-check.sh -p computer-agent-frontend-tauri` stays green
//! without `libwebkit2gtk`. CI installs the system deps, then builds with the
//! feature enabled.

#[cfg(feature = "tauri-app")]
use std::sync::Arc;

#[cfg(feature = "tauri-app")]
use computer_agent_engine::Engine;

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn agent_run_cmd(
    engine: tauri::State<'_, Arc<Engine>>,
    prompt: String,
    session_id: Option<String>,
) -> Result<String, String> {
    let result = super::commands::agent_run(&engine, prompt, session_id)
        .await
        .map_err(|err| format!("{err:#}"))?;
    serde_json::to_string(&result).map_err(|err| format!("{err:#}"))
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn system_info_cmd() -> Result<String, String> {
    let result = super::commands::system_info()
        .await
        .map_err(|err| format!("{err:#}"))?;
    serde_json::to_string(&result).map_err(|err| format!("{err:#}"))
}

/// Run the Tauri event loop. Blocks until the last window closes.
/// Intended for the `src-tauri` binary / `tauri dev`; never called in tests.
#[cfg(feature = "tauri-app")]
pub fn run_app(engine: Arc<Engine>) {
    tauri::Builder::default()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![agent_run_cmd, system_info_cmd])
        .run(tauri::generate_context!())
        .expect("Tauri application must start");
}
