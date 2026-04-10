---
name: agent-cli-reference
description: Use when constructing CLI invocations for coding agents, mapping flags between agents, or troubleshooting agent dispatch. Contains expanded help text for claude, codex, cline, copilot, gemini, grok, and junie.
---

# Agent CLI Reference

Complete help text and quick-reference tables for every supported coding agent CLI. Load the companion file `agent-cli-help.txt` (in this skill directory) for the full expanded output of each CLI's `--help`.

## When to use this skill

- Building or modifying agent dispatch logic (headless flags, model flags, output formats)
- Mapping equivalent capabilities across agents (e.g. "what is Codex's equivalent of Claude's `--dangerously-skip-permissions`?")
- Debugging agent subprocess invocations
- Adding support for a new agent CLI

## Supported agents

| Agent   | Binary    | Headless flag                | Model flag          | Auto/YOLO flag                                   |
|---------|-----------|------------------------------|---------------------|--------------------------------------------------|
| Claude  | `claude`  | `-p` / `--print`             | `--model <m>`       | `--dangerously-skip-permissions`                 |
| Codex   | `codex`   | `exec [--json]`              | `-c model="<m>"`    | `--dangerously-bypass-approvals-and-sandbox`     |
| Cline   | `cline`   | positional arg               | `cline auth -m`     | `--no-interactive` / `--yolo`                    |
| Copilot | `copilot` | suggest/explain subcommands  | (none)              | (none)                                           |
| Gemini  | `gemini`  | `-p` / `--prompt`            | `-m <m>`            | `--yolo`                                         |
| Grok    | `grok`    | `-p` / `--prompt`            | `-m <m>`            | `--sandbox`                                      |
| Junie   | `junie`   | `--task <t>` / positional    | `--model <m>`       | `--brave`                                        |
| xAI     | `copilot` | proxies copilot              | `COPILOT_MODEL` env | `--yolo`                                         |

## Output formats

| Agent   | Formats                          | Default |
|---------|----------------------------------|---------|
| Claude  | text, json, stream-json          | text    |
| Codex   | text, JSONL (exec --json)        | text    |
| Cline   | rich, json, plain                | rich    |
| Copilot | text only                        | text    |
| Gemini  | text, json, stream-json          | text    |
| Grok    | text, json (--format json)       | text    |
| Junie   | text, json, json-stream          | text    |

## Full help text

See `references/agent-cli-help.txt` for the complete `--help` output of every agent CLI including all subcommands. If the file is missing, generate it:

```sh
node .agents/skills/agent-cli-reference/scripts/generate-agent-cli-help.js
```

This requires all agent binaries (claude, codex, cline, copilot, gemini, junie, grok) to be installed and on `$PATH`.
