#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$project_root"
command -v git >/dev/null
command -v python3 >/dev/null
command -v node >/dev/null
node_major="$(node -p 'Number(process.versions.node.split(".")[0])')"
if (( node_major < 22 )); then
  echo 'Configure Node.js 22+ in the cloud environment installation.' >&2
  exit 1
fi
if command -v rustup >/dev/null; then
  rustup toolchain install 1.91.1 --profile minimal --component rustfmt
else
  echo 'Rust compilation stays on GitHub Actions; configure Rust 1.91.1/rustfmt for formatting if needed.'
fi
python3 utils/generate_cloud_fixtures.py --out work/fixtures
node utils/test_browser_helpers.cjs
echo 'Portable setup prepared. No private font, Windows path, or local-PC session is required.'
