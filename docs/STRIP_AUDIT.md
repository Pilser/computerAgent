# Phase 10 — Strip Audit: confirm no `ui/` / `setup/` / `startup` / `print` / `cli` ports

Date: 2026-10-12. Scope: `crates/` vs `_vendor/embedded-agent/` (zerostack 1.8.4 baseline).
Method: `grep -rn` per banned pattern over `crates/`, `Cargo.toml` dep inspection,
`wc -l` size ranking, vendor `find` inventory. No code touched (this doc is the only write).

**Verdict: PASS — zero banned ports. `_vendor` MUST NOT be deleted (no parity yet).**

## 1. Banned-import results (expect zero — confirmed zero)

| # | Pattern | Result |
|---|---------|--------|
| 1 | `zerostack::ui` | ZERO HITS |
| 2 | `::ui::` | ZERO HITS |
| 3 | `setup::` | ZERO HITS |
| 4 | `startup` (case-insensitive) | ZERO HITS |
| 5 | `print::` | ZERO HITS |
| 6 | `cli::` | ZERO HITS |
| 7 | `crossterm` | ZERO HITS |
| 8 | `ratatui` | ZERO HITS |
| 9 | `clap` (incl. `clap::Parser`) | ZERO HITS |
| 10 | `slint` crate use (`use slint`, `slint::`) | ZERO HITS — only doc-comment "`slint` crate dependency yet" in `frontend-slint/src/app.rs:3` and the package name `computer-agent-frontend-slint` |
| 11 | `tauri` crate use (`use tauri`, `tauri::`) | ZERO CODE HITS — one doc-comment `` `#[tauri::command]` `` in `frontend-tauri/src/commands.rs:6` (explicitly notes the attributes land with the future Tauri dependency); package name `computer-agent-frontend-tauri` only |
| 12 | `std::process::Command` outside sys-tools | ZERO HITS — `std::process` appears only as `std::process::id()` in test temp-dir helpers (`session-store/tests/*`, `agent/tools/read.rs:195`, `write.rs:100`, `sys-tools/fs/mod.rs:15`) and as the return type `std::process::Output` in `sys-tools/sandbox/wrap.rs:44` (inside the one allowed crate) |
| 13 | `tokio::fs` / `tokio::process` outside sys-tools + agent/tools | ZERO VIOLATIONS — `tokio::process::Command` only in `sys-tools/sandbox/{wrap,bwrap,zerobox}.rs`; `tokio::fs` only in `sys-tools/fs/{read,write}.rs` and `agent/tools/{read,write}.rs` (both allow-listed locations) |

Banned files check: `find crates \( -path '*ui*' -o -name 'setup*' -o -name 'startup*' -o -name 'print*' -o -name 'cli*' \)` → **no matches**. No `ui/`, `setup/`, `startup.rs`, `print.rs`, `cli.rs` anywhere under `crates/`.

## 2. Dependency directions (one-way, no cycles)

`Cargo.toml` deps per crate:

| Crate | Depends on (workspace-external: serde/serde_json/thiserror/tokio/regex/chrono/uuid/anyhow) + intra-workspace |
|---|---|
| `policy` | none intra-workspace |
| `sys-tools` | none intra-workspace |
| `session-store` | none intra-workspace |
| `agent` | `sys-tools`, `policy` |
| `command-api` | none intra-workspace (**no engine dep** — confirmed in both `Cargo.toml` and sources; only a `TODO(phase8)` comment in `dispatch.rs:19`) |
| `engine` | `command-api`, `agent`, `policy`, `session-store`, `sys-tools` |
| `frontend-slint` | `command-api`, `engine` |
| `frontend-tauri` | `command-api`, `engine` |

Import-direction checks (`grep -rn` over sources):

- Frontends → `sys-tools` / `agent` / `policy` / `session-store` internals: **ZERO HITS** (AGENTS.md §4.7 reject-rule passes).
- Frontends → `std::process` / `tokio::fs` directly: **ZERO HITS** (only `std::path::PathBuf` + `std::fs` in test helpers).
- `engine` → `frontend-*`: **ZERO CODE HITS** (only doc words "frontend" in `turns/stream.rs`, `turns/run.rs:80`, `error.rs:30` describing the fan-out/subscriber direction).
- `command-api` → `engine`: **ZERO HITS**.

OBSERVATION (not a failure): both frontends import `computer_agent_engine::Engine`
(`frontend-slint/src/{app,bridge}.rs`, `frontend-tauri/src/commands.rs`) and declare the
`computer-agent-engine` dep, using `engine.dispatch(...)` for `AgentRun` while `SystemInfo`
goes through `command-api::dispatch`. AGENTS.md §4.7 allows this (its reject-rule covers only
sys-tools/agent internals and raw `std::process`/`tokio::fs`). ARCHITECTURE.md §2 leans
dispatch-only ("Frontends never import engine/sys/agent directly ... except `command-api` +
`engine` handle type"); converging `AgentRun` onto `command-api::dispatch` once the engine
seam supports it is recorded as future work, not a phase-10 defect.

## 3. Top-10 largest files in `crates/` (all ≤ 500 — `check-bloat.sh --max 500` passes)

Total: 78 `.rs` files, 7151 lines.

| Rank | Lines | File |
|------|------:|------|
| 1 | 298 | `crates/policy/src/checker.rs` |
| 2 | 295 | `crates/session-store/src/memory/search.rs` |
| 3 | 287 | `crates/agent/src/tools/read.rs` |
| 4 | 277 | `crates/session-store/tests/memory.rs` |
| 5 | 269 | `crates/agent/src/provider/traits.rs` |
| 6 | 264 | `crates/agent/src/tools/bash.rs` |
| 7 | 255 | `crates/session-store/src/session/messages.rs` |
| 8 | 252 | `crates/engine/src/turns/run.rs` |
| 9 | 227 | `crates/session-store/src/memory/store.rs` |
| 10 | 215 | `crates/session-store/src/session/types.rs` |

Headroom to the 500-line gate: largest file is at 60% (298/500). Next file below top-10 is
`session/storage.rs` (183). Bloat gate output: `bloat check ok (max 500 lines)`.

## 4. Ported-vs-remaining matrix (vendor → crates)

PORTED (behavior ported, tested):

| Vendor source | Port destination |
|---|---|
| `permission/checker.rs` (+ pattern) | `policy/{checker,pattern,decision,store,doom_loop,defaults}.rs` |
| `session/{mod,storage,chat_history}.rs` | `session-store/session/{types,messages,storage,compaction,chat_history}.rs` |
| `extras/memory/mod.rs` | `session-store/memory/{store,write,edit,search,inject}.rs` (32 KiB inject / 64 KiB write caps kept) |
| `sandbox.rs` | `sys-tools/sandbox/{types,wrap,masks,network,bwrap,zerobox}.rs` (OFF by default) |
| `fs.rs` | `sys-tools/fs/{read,write,watch?/mod}.rs` (+ os_info) |
| `agent/tools/{read,write,bash}.rs` | `agent/tools/{read,write,bash,registry,mod}.rs` (bash via sandbox wrap) |
| `provider.rs` (shapes/URLs/catalog/pricing) | `agent/provider/{traits,openai,anthropic,gemini,ollama,openrouter,catalog}.rs` — **payload builders + catalog only, no HTTP client** (`provider/mod.rs:4`: "WITHOUT `rig` / `reqwest`" ) |
| `agent/runner.rs` (structure) | `agent/runner/{turn,events}.rs` — **stub runner** returning deterministic `TurnEvent`s |
| `agent/prompt.rs` | `agent/prompt/{system,assemble}.rs` |
| `engine/{mod,stall}.rs` (structure) | `engine/turns/{run,stream,cancel}.rs` (stub turn wiring) |
| `models_catalog.rs`, `pricing.rs` | `agent/provider/catalog.rs` (`context_window`, `estimate_cost`) |

NOT YET PORTED — explicit FUTURE milestones (each must be a phased port, never a copy of `ui/`):

| Milestone | Vendor source | Note |
|---|---|---|
| Real LLM HTTP | `provider.rs` execution half, `agent/builder.rs` | `reqwest`/`rig` client, key resolution → request → response; currently URL/payload shapes only |
| Streaming | `agent/runner.rs` loop, `engine/sink.rs` | token/tool-event stream loop; currently stub `TurnEvent::Text/Done` + in-process fan-out hub |
| Subagents | `extras/subagents/*`, `agent/tools` task tool | only the string `"subagent"` in an allow-list (`policy/checker.rs:238`); no task tool, no builder |
| MCP | `extras/mcp/*` | `checker.rs:5` explicitly notes "no `mcp`/`hooks` branches" — trimmed, not ported |
| LSP | `extras/lsp/*`, `agent/tools/lsp.rs` | no client/registry/rpc, no tool |
| ACP | `extras/acp/*` | untouched |
| Advisor | `extras/advisor/*` | untouched |
| Hooks | `extras/hooks/*` | untouched (see MCP note) |
| Multimodal | `extras/multimodal/*`, `agent/image_relay.rs`, `agent/tools/read_image.rs` | untouched; `TurnEvent` has only a text-delta variant |
| Headless loop | `extras/loop/*` | untouched |
| git_worktree | `extras/git_worktree/*` | untouched |
| archmd | `extras/archmd/*` | untouched |
| chain / export / truncate / rtk / status | `extras/chain/*`, `extras/export.rs`, `extras/truncate.rs`, `extras/rtk.rs`, `extras/status_signals.rs`, `context/*` | untouched |
| Remaining tools | `agent/tools/{edit,grep,find_files,list_dir,todo,crc,normalize}.rs`, `memory_tools` | `registry.list()` is `["read", "write", "bash"]`; `edit`/`grep`/`find_files`/`list_dir`/`todo` not yet ported (only named in checker allow-lists) |
| Config | `config/*` | no `agent/config/` yet |
| Auth / retry / logging / docs / event | `auth.rs`, `retry.rs`, `logging.rs`, `docs.rs`, `event.rs` | untouched |
| Engine remainder | `engine/{ask_freeze,sink}.rs` | untouched |
| Permission ask flow | `permission/ask.rs` | `Decision::Ask` exists; interactive ask flow not ported |
| setup | `setup/*` | **BANNED from porting 1:1** — installer/onboarding stays out; future equivalent (if any) is a new `command-api` command, not a vendor copy |
| startup | `startup.rs` | **BANNED** — same rule as setup |
| print | `print.rs` | **BANNED** — TUI print path; frontends render via views, never via a print port |
| cli | `cli.rs`, `main.rs` | **BANNED** — clap CLI stays out; entry points are the two GUI shells |
| ui | `ui/*` (33 files: app, renderer, events, feed, input, pickers, slash/*, statusline, ...) | **BANNED** — ratatui/crossterm TUI is never ported; `frontend-slint` + `frontend-tauri` views replace it on the `command-api` contract |

## 5. DO-NOT-DELETE-`_vendor` verdict

**`_vendor/embedded-agent/` MUST BE KEPT as the read-only reference. Deletion is REJECTED at phase 10.**

Reasons:

1. **No parity.** §4 lists ~30 unported modules. The ported surface is payload shapes, stub
   runner, policy checker, session/memory stores, fs/sandbox wrappers, and 3 of ~10 tools.
   Per ARCHITECTURE.md §0/§7.10, `_vendor` goes only "after parity" — parity does not exist.
2. **Real LLM path missing.** No HTTP client, no streaming loop, no subagents/MCP — the
   vendor files are still the only executable reference for that behavior when those
   milestones are scheduled.
3. **Port source for every future milestone.** Each row in §4's NOT-PORTED table names the
   vendor file to port from. Deleting the reference before porting it strands that work.
4. **Zero cost to keeping it.** The audit proves (`§1`) no banned module leaked into
   `crates/`; `_vendor` is not a workspace member and ships in no crate. Keeping it changes
   no build artifact (release profile builds `crates/*` only via CI `release.yml`).
5. **Delete condition restated:** `_vendor` may be removed only when a later audit shows
   (a) real-LLM + streaming + agreed tool/MCP scope ported with tests, (b) this report's
   banned-import + dep-direction + bloat checks still green, and (c) an explicit user
   approval names the commit. None hold today.

## 6. Validation

- `cargo fmt --check` → **PASS** (exit 0, no diff). Allowed read-only check; no code changed.
- `./scripts/check-bloat.sh --max 500` → **PASS** (`bloat check ok (max 500 lines)`).
- `cargo check` / `cargo test`: **NOT RUN** — no code changes in this phase, so per-crate
  wrappers were not required (AGENTS.md §2 triggers only on code changes). Last code state
  is untouched; this commit adds `docs/STRIP_AUDIT.md` only.
