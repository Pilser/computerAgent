#!/usr/bin/env bash
# Download the CI-built `computer-agent` desktop binary into deploy/.
# Usage: ./fetch-binary.sh [tag]   (default: v0.3.0)
# Needs a token with repo read access: set COMPUTERAGENT_TOKEN,
# or it falls back to the local git credential store.
set -euo pipefail
TAG="${1:-v0.3.0}"
HERE="$(cd "$(dirname "$0")" && pwd)"

token() {
  if [ -n "${COMPUTERAGENT_TOKEN:-}" ]; then
    printf '%s' "$COMPUTERAGENT_TOKEN"
  else
    sed -n 's#.*://[^:]*:\([^@]*\)@.*#\1#p' ~/.git-credentials | head -1
  fi
}

TKN="$(token)"
if [ -z "$TKN" ]; then
  echo "No token: set COMPUTERAGENT_TOKEN or store github.com credentials first." >&2
  exit 1
fi

RUN_ID="$(curl -sf -H "Authorization: Bearer $TKN" \
  "https://api.github.com/repos/Pilser/computerAgent/actions/runs?per_page=5" \
  | python3 -c "
import json, sys
runs = json.load(sys.stdin)['workflow_runs']
good = [r for r in runs if r['head_branch'] == '$TAG' and r['conclusion'] == 'success' and r['name'] == 'release']
print(good[0]['id'] if good else '')
")"
if [ -z "$RUN_ID" ]; then
  echo "No successful run found for tag $TAG." >&2
  exit 1
fi

ART_ID="$(curl -sf -H "Authorization: Bearer $TKN" \
  "https://api.github.com/repos/Pilser/computerAgent/actions/runs/$RUN_ID/artifacts" \
  | python3 -c "
import json, sys
arts = json.load(sys.stdin)['artifacts']
print(arts[0]['id'] if arts else '')
")"
if [ -z "$ART_ID" ]; then
  echo "Run $RUN_ID has no artifacts (libs-only build?)." >&2
  exit 1
fi

curl -sL -H "Authorization: Bearer $TKN" \
  "https://api.github.com/repos/Pilser/computerAgent/actions/artifacts/$ART_ID/zip" \
  -o "$HERE/bundle.zip"
rm -f "$HERE/computer-agent"
unzip -o -q "$HERE/bundle.zip" -d "$HERE"
rm -f "$HERE/bundle.zip"
chmod +x "$HERE/computer-agent"
echo "Ready: $HERE/computer-agent"
file "$HERE/computer-agent"
echo "Launch it with: $HERE/computer-agent  (see README.md)"
