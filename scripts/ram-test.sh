#!/usr/bin/env bash
# Usage: ./scripts/ram-test.sh -p <crate> [extra cargo args...]
# ALWAYS scope to one crate (-p). NEVER --workspace (see AGENTS.md §2).
# Full-workspace test is allowed ONLY at integration phases (8+).
set -euo pipefail
REQ_SHM_MB=1024
REQ_AVAIL_MB=2048
avail_shm_kb=$(df -k /dev/shm | awk 'NR==2 {print $4}')
avail_mem_kb=$(free -k | awk '/^Mem:/ {print $7}')
if [ "$avail_shm_kb" -lt $((REQ_SHM_MB * 1024)) ] || [ "$avail_mem_kb" -lt $((REQ_AVAIL_MB * 1024)) ]; then
  echo "REFUSING: need >=${REQ_SHM_MB}MiB free in /dev/shm and >=${REQ_AVAIL_MB}MiB avail RAM."
  echo "shm free: $((avail_shm_kb / 1024))MiB, mem avail: $((avail_mem_kb / 1024))MiB. Close apps or defer to CI."
  exit 1
fi
export CARGO_TARGET_DIR=/dev/shm/ca-target
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-C codegen-units=4"
mkdir -p "$CARGO_TARGET_DIR"
if [ "$#" -eq 0 ]; then
  echo "Usage: $0 -p <crate> [...]. Refusing unscoped run (see AGENTS.md §2)."
  exit 1
fi
for a in "$@"; do
  if [ "$a" = "--workspace" ]; then
    echo "REFUSING --workspace: scope to one crate with -p (integration phases 8+ only)."
    exit 1
  fi
done
exec cargo test --jobs 2 "$@" -- --test-threads=2
