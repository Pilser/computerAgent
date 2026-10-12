#!/usr/bin/env bash
# Fail if any .rs file exceeds max lines (default 500). Excludes _vendor and target.
set -euo pipefail
MAX="${1:---max}"
if [ "$MAX" = "--max" ]; then MAX="${2:-500}"; else MAX="500"; fi
OVER=$(find . -path ./_vendor -prune -o -path ./target -prune -o -name '*.rs' -print | xargs wc -l | awk -v m="$MAX" '$1>m {print}')
if [ -n "$OVER" ]; then
  echo "BLOAT CHECK FAILED (max $MAX lines):"
  echo "$OVER"
  exit 1
fi
echo "bloat check ok (max $MAX lines)"
