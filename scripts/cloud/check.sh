#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$project_root"
bash -n scripts/cloud/setup.sh scripts/cloud/check.sh
python3 -m py_compile utils/generate_cloud_fixtures.py utils/verify_portable.py scripts/cloud/preview.py
python3 utils/generate_cloud_fixtures.py --out work/fixtures
node utils/test_browser_helpers.cjs
echo 'Static/helper checks passed. Run Rust/native/WASM through GitHub Actions.'
