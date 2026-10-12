# Architecture — computerAgent

Full-privilege computer assistant (Windows / Linux / macOS) with embedded AI.
Flow: `frontend-slint | frontend-tauri -> command-api -> engine -> agent + sys-tools`, guarded by `policy`, persisted by `session-store`.

Decisions locked in this doc: sandbox OFF by default, memory KEPT, dual frontends on one command layer, in-process v1, no file >500 lines.

## 0. Base import

- `_vendor/embedded-agent/` = zerostack 1.8.4, `.git`/`.github` stripped, committed as baseline. Read-only reference. NEVER a workspace member. NEVER copy `src/ui/`.
- The vendored copy stays COMPLETE (not file-stripped): 12 of its files reference `ui::` helpers (`session`, `engine/sink`, `agent/builder`, …), so deleting `src/ui/` would break the reference lib. Physical TUI removal is fork-side surgery, tracked as a milestone (private `Pilser/embedded-agent` repo reserved for the stripped dependency).
- All new code lives in `crates/*`. Port is file-by-file, phase-by-phase via subagents (see `AGENTS.md` phases). Delete `_vendor` only after parity.

## 1. Folder structure (target tree, nothing scaffolded yet)

```text
computerAgent/
  AGENTS.md               # build/test rules (RAM-only, CI-only release)
  docs/ARCHITECTURE.md    # this file
  scripts/
    check-bloat.sh        # fail if any crates/**/*.rs > 500 lines
    ram-check.sh          # wrapper: sets RAM target dir + job caps, then cargo check
    ram-test.sh           # wrapper: sets RAM target dir + job caps, then cargo test
  .github/workflows/
    ci.yml                # check + test (linux) on PR
    release.yml           # ONLY place that runs --release (CI, capped jobs)
  _vendor/embedded-agent/ # reference only
  crates/
    command-api/src/
      lib.rs              # re-export only
      types.rs            # Command, CommandResult (serializable, versioned)
      types/
        mod.rs
        agent_cmds.rs     # AgentRun, AgentCancel, AgentStatus
        sys_cmds.rs       # SystemInfo, ShellExec, FileRead, FileWrite
        session_cmds.rs   # SessionOpen, SessionClose, MemorySearch
      dispatch.rs         # async dispatch(cmd) -> engine (policy inside, never frontend)
      error.rs            # CommandError
    engine/src/
      lib.rs
      orchestrator.rs     # Engine::handle() delegates only, <200 lines
      error.rs
      turns/
        mod.rs
        run.rs            # single turn: prompt assemble -> runner -> store
        stream.rs         # token/tool event fan-out to frontends
        cancel.rs
      stall/
        mod.rs
        detect.rs
        recover.rs
    agent/src/            # slimmed zerostack, NO ui/
      lib.rs
      error.rs
      provider/
        mod.rs
        traits.rs         # AnyClient/AnyModel dispatch enums
        openai.rs
        anthropic.rs
        gemini.rs
        ollama.rs
        openrouter.rs
        catalog.rs        # models_catalog + pricing
      runner/
        mod.rs
        spawn.rs
        events.rs
        compaction_hook.rs
      prompt/
        mod.rs
        system.rs         # static instructions (NOT memory)
        assemble.rs       # system + memory_block + context files
      tools/              # LLM-callable tools (model -> system), 1 file per tool
        mod.rs            # registry re-export only
        registry.rs       # ToolRegistry, profile gating (coding vs sysadmin)
        read.rs           # port vendor read.rs
        write.rs          # port vendor write.rs
        edit.rs           # port vendor edit.rs
        bash.rs           # port vendor bash.rs -> via sys-tools/sandbox
        grep.rs
        find_files.rs
        list_dir.rs
        todo.rs
        memory_tools.rs   # memory_write/read/edit/search -> session-store
      config/
        mod.rs
        types.rs
        load.rs
    sys-tools/src/        # full-privilege OS ops, ONLY crate that touches OS raw
      lib.rs
      error.rs
      fs/
        mod.rs
        read.rs
        write.rs
        watch.rs
        fs_windows.rs
        fs_linux.rs
        fs_macos.rs
      process/
        mod.rs
        spawn.rs
        kill.rs
        list.rs
      package/
        mod.rs
        detect.rs         # winget / apt-dnf-pacman / brew
        install.rs
      service/
        mod.rs
        systemd.rs
        launchd.rs
        windows_svc.rs
      os_info/
        mod.rs
        info.rs
      sandbox/            # KEPT but OFF by default (see §4)
        mod.rs
        types.rs          # Sandbox { enabled:false, required:false, backend, network:true }
        wrap.rs           # wrap_command / output_command (port vendor sandbox.rs split)
        masks.rs          # credential masks (.ssh .aws .gnupg...)
        network.rs        # --unshare-net hints
        bwrap.rs          # linux backend args
        zerobox.rs        # macos backend args
    policy/src/
      lib.rs
      decision.rs         # Allow / Ask / Deny
      checker.rs          # glob+regex port of permission/checker.rs
      store.rs            # allowlist persist
      doom_loop.rs        # repeated-call detection
    session-store/src/
      lib.rs
      session/
        mod.rs
        types.rs          # Session, SessionMessage, ToolRecord
        messages.rs       # add_message, tool_call/result, truncate, rewind
        compaction.rs     # select_cut, compress, needs_compaction
        storage.rs        # JSON load/save
        chat_history.rs   # jsonl append
      memory/             # KEPT (§3), injected into prompt, NOT the prompt itself
        mod.rs
        store.rs          # Mem { root, project, today }, list/read
        write.rs
        edit.rs
        search.rs
        inject.rs         # context_block() -> <memory>...</memory>
    frontend-slint/src/
      lib.rs
      app.rs              # headless dispatch proofs (no window, test-safe)
      window.rs           # REAL Slint window: AppWindow + send callback -> engine
      bridge.rs           # callbacks -> command-api::dispatch() ONLY
      views/
        mod.rs            # chat.slint, approval.slint, settings.slint land here
    frontend-tauri/src/
      lib.rs
      app.rs              # REAL tauri::Builder, --features tauri-app only
      commands.rs         # #[tauri::command] -> dispatch() SAME enum
      web/
        mod.rs            # api.ts (same Command JSON, enables cloud reuse)
```

Dependency direction (one-way, no cycles):

```text
frontend-slint, frontend-tauri -> command-api -> engine -> { agent, sys-tools, policy, session-store }
agent -> { sys-tools, policy, session-store }
engine -> { agent, sys-tools, policy, session-store }
nobody -> frontend-*
```

## 2. Command layer (both frontends, one contract)

```rust
pub enum Command { AgentRun{..}, AgentCancel, SystemInfo, ShellExec{..}, FileRead{..}, FileWrite{..}, SessionOpen(..), MemorySearch{..} }
pub async fn dispatch(cmd: Command) -> Result<CommandResult, CommandError>
```

- v1 in-process: `Engine` built once, shared via `Arc`. Frontends never import engine/sys/agent directly (enforced by review, no direct dep in frontend `Cargo.toml` except `command-api` + `engine` handle type).
- Cloud later: same `Command` over Tauri IPC / HTTP. No frontend rewrite.
- Policy enforced inside `dispatch`/engine, never in UI.

## 3. Memory (KEPT) vs session vs system prompt

- `system prompt` (`agent/prompt/system.rs`): static role + tool schemas. No user data.
- `session` (`session-store/session/`): short-term current conversation. Dies on `/clear`. Port of `session/mod.rs:132`.
- `memory` (`session-store/memory/`): long-term across reboots. `MEMORY.md` global facts + per-project `SCRATCHPAD.md` open items + `daily/*.md` last-2 logs + `notes/*.md` searchable refs. Assembled by `inject.rs::context_block()` into `<memory>…</memory>` appended to preamble. Port of `extras/memory/mod.rs:159/581`. Capped at 32 KiB inject / 64 KiB write.

## 4. Sandbox (KEPT, OFF by default)

Port of `src/sandbox.rs` split into `sys-tools/sandbox/*.rs`. Wraps every `bash` tool call with `bwrap` (Linux) / `zerobox` (macOS).

- Default: `enabled = false`, `required = false`, `network = true` → commands run bare = full-privilege assist mode (configure system, install, touch user files).
- Safe mode opt-in: `enabled = true` → credential masks + optional `--unshare-net` + `required=true` refuses instead of running bare when backend missing.
- `bash.rs` tool is the ONLY caller of `wrap_command()`. No other tool bypasses it.

## 5. Tools (LLM-callable, model -> system)

Each `agent/tools/<name>.rs` implements one `Tool { NAME, Args, call() }`, gated by `check_perm()` then `policy`. Distinct from `command-api::Command` (human -> system):

```text
UI --Command--> engine --prompt--> LLM --tool call--> registry --> check_perm --> sys-tools/session-store --> LLM --> UI
```

`read/write/edit/list_dir/grep/find_files/todo` port 1:1 from vendor `agent/tools/`. `bash` ports via sandbox. `memory_tools` bridges to `session-store`. Future OS tools (`open_app`, `system_setting`) follow the same single-file pattern.

UI tools: frontends expose NO tools directly. Approval dialogs (`Ask`) come from `policy` events over `stream.rs`, rendered by `frontend-*/views`, answered back as `Command::Approve/Deny`.

## 6. Anti-bloat (hard gate)

- 500 lines max, 300 target per `.rs`. `lib.rs`/`mod.rs` re-export only.
- Split before 400: `provider.rs` (1562) → `provider/*.rs`; `engine/mod.rs` (2464) → `turns/*, stall/*`; `sandbox.rs` (1014) → `sandbox/*.rs`; `memory/mod.rs` (1275) → `memory/*.rs`.
- `scripts/check-bloat.sh --max 500` runs in CI + pre-commit. No exceptions without doc note.

## 7. Phases (subagents, sequential — see AGENTS.md for build caps)

1. Workspace + `command-api` + bloat gate + CI skeletons.
2. `policy` (checker port) + tests.
3. `session-store/session` (types/messages/storage) + tests.
4. `session-store/memory` (store/write/search/inject) + tests.
5. `sys-tools/fs + os_info` + `sandbox` (OFF default) + tests.
6. `agent/tools/read+write+bash+registry` wired to sys-tools + tests.
7. `agent/provider` split + `runner` slim + `prompt/assemble` + tests.
8. `engine/turns` wiring `dispatch(AgentRun)` end-to-end + tests.
9. `frontend-slint` + `frontend-tauri` thin shells calling `dispatch(SystemInfo, AgentRun)`.
10. Strip audit: confirm no `ui/`, `setup/`, `startup.rs`, `print.rs`, `cli.rs` ported; remove `_vendor` after parity.
11. Real backends: LLM HTTP in `agent/provider` (reqwest), streaming runner, `edit/grep/find/todo` tools, Slint views polish, Tauri system-deps CI job (`--features tauri-app`), fork-side strip into `Pilser/embedded-agent` as git dependency.
