#!/usr/bin/env node

const { execFileSync } = require("child_process");
const { writeFileSync } = require("fs");
const { resolve, join } = require("path");

const S = "=".repeat(80);
const agents = [
  ["CLAUDE CODE", "claude", ["auth", "mcp"], ["--version"]],
  ["CODEX CLI", "codex", ["exec"], ["--version"]],
  ["CLINE CLI", "cline", ["auth", "task", "config"], ["version"]],
  ["GITHUB COPILOT CLI", "copilot", [], ["--version"]],
  ["GEMINI CLI", "gemini", ["mcp", "extensions", "skills", "hooks"], ["--version"]],
  ["JUNIE CLI", "junie", [], ["--version"]],
  ["GROK CLI", "grok", ["git", "mcp"], ["--version"]],
];

const run = (args) => {
  try { return execFileSync(args[0], args.slice(1), { encoding: "utf8", timeout: 10000 }).trim(); }
  catch (e) { return (e.stdout || e.stderr || "").trim(); }
};

let out = `${S}\nAGENT CLI REFERENCE\n${S}
Expanded help text for every supported coding agent CLI. Use this as a lookup
when mapping agent flags, capabilities, and invocation patterns.

Generated: ${new Date().toISOString().slice(0, 10)}\n\n`;

for (const [label, bin, subs, verArgs] of agents) {
  out += `${S}\n${label}  (${bin})\nVersion: ${run([bin, ...verArgs])}\nBinary:  ${bin}\n${S}\n\n${run([bin, "--help"])}\n\n`;
  for (const sub of subs) out += `--- ${bin} ${sub} ---\n\n${run([bin, sub, "--help"])}\n\n`;
}

out += `${S}
QUICK REFERENCE: HEADLESS / NON-INTERACTIVE FLAGS
${S}

Agent       Headless flag                              Model flag             Auto/YOLO flag
----------  ----------------------------------------   --------------------   ---------------------------------
claude      -p / --print                               --model <m>            --dangerously-skip-permissions
codex       exec [--json]                              -c model="<m>"         --dangerously-bypass-approvals-and-sandbox
cline       (prompt as positional arg)                 (via cline auth -m)    --no-interactive / --yolo
copilot     (suggest/explain subcommands)              (none)                 (none)
gemini      -p / --prompt                              -m <m>                 --yolo
grok        -p / --prompt                              -m <m>                 --sandbox
junie       --task <t> / positional arg                --model <m>            --brave
xai         (proxies copilot with COPILOT_MODEL env)   COPILOT_MODEL env      --yolo

${S}
QUICK REFERENCE: MCP SERVER MANAGEMENT
${S}

Agent       Add command
----------  ----------------------------------------------------------------
claude      claude mcp add <name> <commandOrUrl> [args...]
codex       codex mcp add <name> <commandOrUrl> [args...]
cline       (configured via cline config)
copilot     (no MCP support)
gemini      gemini mcp add <name> <commandOrUrl> [args...]
grok        grok mcp add <name> / grok mcp add-json <name> <json>
junie       --mcp-location=<path> (folder-based, no add subcommand)

${S}
QUICK REFERENCE: OUTPUT FORMATS
${S}

Agent       Formats available                          Default
----------  ----------------------------------------   ---------
claude      text, json, stream-json                    text
codex       text (interactive), JSONL (exec --json)    text
cline       rich, json, plain                          rich
copilot     text only                                  text
gemini      text, json, stream-json                    text
grok        text (interactive), json (--format json)   text
junie       text, json, json-stream                    text
`;

writeFileSync(resolve(join(__dirname, "..", "references", "agent-cli-help.txt")), out);
console.log("Done.");