#!/usr/bin/env node
/**
 * Intentionally small CLI whose contract can change across semver majors.
 *
 * 1.x: `--version`, `-V`, and `version` subcommand all print the package version.
 * For a Codex drill, publish 2.x that drops `--version` / `-V` so CI must add a
 * shell fallback (see package README).
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgPath = join(__dirname, "..", "package.json");
const { version } = JSON.parse(readFileSync(pkgPath, "utf8"));
const major = Number.parseInt(String(version).split(".")[0], 10) || 1;

const argv = process.argv.slice(2);

if (argv.length === 0) {
  console.error("freq-ai-cli-compat-fixture: missing args (try --help)");
  process.exit(2);
}

const wantHelp =
  argv[0] === "--help" ||
  argv[0] === "-h" ||
  argv.includes("--help");

const wantVersionFlag = argv.includes("--version") || argv.includes("-V");
const wantVersionCmd = argv[0] === "version";

if (wantHelp && !wantVersionCmd && !wantVersionFlag) {
  console.log(`freq-ai-cli-compat-fixture ${version}
Usage:
  freq-ai-cli-compat-fixture --help
  freq-ai-cli-compat-fixture --version   (1.x only; removed in 2.x drill)
  freq-ai-cli-compat-fixture version`);
  process.exit(0);
}

const allowLegacyVersionFlags = major < 2;

if (wantVersionCmd || wantVersionFlag) {
  if (!allowLegacyVersionFlags && wantVersionFlag) {
    console.error(
      "freq-ai-cli-compat-fixture: unknown flag (2.x drill: use the `version` subcommand)",
    );
    process.exit(1);
  }
  console.log(version);
  process.exit(0);
}

// Best-effort success for any other argv (extra probes stay non-fatal).
process.exit(0);
