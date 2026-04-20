# freq-ai-cli-compat-fixture

Published npm CLI used by **CLI Compat** to simulate real-world flag drift.

**Current drill:** `2.0.0` is published; **2.x** rejects `--version` / `-V` (use the `version` subcommand). **CLI Compat** installs `freq-ai-cli-compat-fixture@2.0.0` from npm and still runs `freq-ai-cli-compat-fixture --version` in verify, so that step fails until the workflow gains a shell fallback (for example Codex autofix).

## Publish

```bash
cd packages/cli-compat-fixture
npm publish --access public
```

You need an npm account and one-time `npm login`.

## Ending the drill

- In `.github/workflows/cli-compat.yml`, change verify to something like  
  `run_logged bash -lc "freq-ai-cli-compat-fixture version || freq-ai-cli-compat-fixture --version"`.
- Optionally pin install to **1.x** or go back to installing from the repo path.

## Re-running a drill later

1. **3.x** (or another major): extend `bin/freq-ai-cli-compat-fixture.mjs` with a new breaking rule, bump `package.json`, `bun publish`.
2. Point CI at the new version and a verify line that only uses the old flags so verify fails, then fix (manually or via Codex autofix).
