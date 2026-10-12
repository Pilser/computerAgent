#!/usr/bin/env bash
# RAM-capped cargo test. ONLY local test entrypoint (see AGENTS.md §2).
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
exec cargo test --workspace --jobs 2 -- --test-threads=2 "$@"
