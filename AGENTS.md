# AGENTS.md — computerAgent build / test / implement rules

These rules are load-bearing. Slow disk + 7.6 GiB RAM: a stray `cargo build --release` freezes this machine. CI builds binaries, this machine only checks logic.

## 1. NEVER on this machine

- NEVER `cargo build`, `cargo run`, `cargo install`, `cargo install --path .`, `cargo package`, `cargo publish`.
- NEVER `--release`, NEVER `--profile release`, NEVER `lto`, NEVER `strip` locally.
- NEVER `cargo bench`, NEVER `cargo fuzz`.
- NEVER two `cargo` processes at once (one check/test at a time — subagents serialize).
- NEVER set `CARGO_TARGET_DIR` to the repo (`./target`) — it hammers the slow disk. ALWAYS RAM (§2).
- Vendor exception: `_vendor/embedded-agent/AGENTS.md` says never `cargo check` there — that rule stays INSIDE `_vendor` only. For `crates/*`, `cargo check` + `cargo test` via §2 wrappers are the ONLY allowed commands.

## 2. ONLY allowed locally: check + test, in RAM, capped

All local cargo runs MUST go through the wrappers (they set target dir + caps):

```bash
./scripts/ram-check.sh   # == cargo check --workspace --tests with RAM caps
./scripts/ram-test.sh    # == cargo test --workspace with RAM caps
./scripts/check-bloat.sh # no caps needed, always allowed
cargo fmt --check        # allowed (no build artifacts)
```

What the wrappers set (do NOT override to be more parallel):

```bash
export CARGO_TARGET_DIR=/dev/shm/ca-target        # RAM tmpfs, never repo ./target
export CARGO_BUILD_JOBS=2                          # 4-core box: leave 2 cores for desktop
export CARGO_INCREMENTAL=0                         # less RAM, slower rebuild is fine
export RUSTFLAGS="-C codegen-units=4"              # cap parallel codegen memory
# invocation: cargo check --workspace --tests --jobs 2
#             cargo test  --workspace --jobs 2 -- --test-threads=2
```

RAM guardrails (wrappers enforce, humans too):

- `/dev/shm` is only ~1.6 GiB. Before running: `df -h /dev/shm` must show ≥1 GiB free, `free -h` must show ≥2 GiB available. Else DO NOT RUN — ask user to close apps or defer to CI.
- If `/dev/shm` is full: `rm -rf /dev/shm/ca-target/*` is allowed; NEVER `rm -rf target/` in repo (there should be no such dir — if one exists, delete it once and never recreate).
- Pre-approved overflow dir `/tmp/opencode/ca-target` (disk, slow) is LAST resort and ONLY if user explicitly says `allow-disk-target`. Even then: `--jobs 1`, one crate at a time.
- If OOM risk (compile stalls, swap climbing per `free`): Ctrl-C immediately, report, defer to CI.

## 3. Binaries ONLY in CI

- `.github/workflows/ci.yml`: `cargo check` + `cargo test` + `check-bloat.sh` on PR. Linux runner, default disk target dir, `--jobs 2`.
- `.github/workflows/release.yml`: the ONLY place that may run `cargo build --release`. Capped: `--jobs 2`, `codegen-units=1`, `lto=thin`, `strip=true`. Artifacts uploaded, never built locally.
- Subagents: NEVER add a workflow that builds releases on PR, NEVER raise job counts to "speed up CI" without a RAM note.

## 4. How subagents implement (phase by phase, sequential)

Phases are in `docs/ARCHITECTURE.md §7`. Rules for every implementation subagent:

1. One phase at a time, in order. Do NOT start phase N+1 until phase N is `ram-check + ram-test + check-bloat` green.
2. Read `docs/ARCHITECTURE.md` + this file + the cited `_vendor` source FIRST. Port, don't reinvent; never port `src/ui/`, `setup/`, `startup.rs`, `print.rs`, `cli.rs`.
3. One domain per file, ≤500 lines hard / ≤300 target. `lib.rs`/`mod.rs` re-export only. New domain = new file.
4. Non-TUI code MUST ship tests (vendor rule inherited). Frontend `views/` are exempt; everything under `agent/`, `engine/`, `policy/`, `session-store/`, `sys-tools/` is not.
5. Sandbox stays `enabled=false` default. Any PR flipping the default is rejected.
6. Memory stays. `memory/` changes must keep the 32 KiB inject / 64 KiB write caps.
7. Frontends may ONLY call `command-api::dispatch()`. A frontend importing `sys-tools`, `agent` internals, or `std::process`/`tokio::fs` directly is rejected.
8. Each phase ends with: `cargo fmt`, `./scripts/ram-check.sh`, `./scripts/ram-test.sh`, `./scripts/check-bloat.sh --max 500`, plus updated `docs/` notes if behavior changed.
9. Commit message prefix: `phase<N>: <crate> - <what>` (e.g. `phase2: policy - port checker`). One phase = one commit. Do NOT commit `_vendor` changes (read-only).
10. If a phase needs a tool the sandbox would block, ASK in the session (post `Ask` event) — do NOT silently disable policy/sandbox to get tests green.

## 5. Quick reference

| Want | Do |
|---|---|
| check types | `./scripts/ram-check.sh` |
| run tests | `./scripts/ram-test.sh` |
| bloat gate | `./scripts/check-bloat.sh --max 500` |
| format | `cargo fmt` (check-only `cargo fmt --check` in CI) |
| local binary | NO — open a PR, let `release.yml` build it |
| disk slow | expected — that is WHY target dir is RAM |
| machine freezing | Ctrl-C, `rm -rf /dev/shm/ca-target`, report, defer to CI |
