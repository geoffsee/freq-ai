#!/usr/bin/env bash
# Top-level local-cicd entrypoint for the model-dig job: install provider CLIs,
# regenerate assets/available-models.json, validate the JSON shape.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

./scripts/ci/install-provider-clis.sh

# scripts/model-dig.sh emits a large raw blob to stdout; the curated output
# is written to assets/available-models.json as a side effect.
./scripts/model-dig.sh > /dev/null

python3 -c "import json; json.load(open('assets/available-models.json'))"
