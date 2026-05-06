# freq-ai

The agentic-undead product team.

- Desktop
- Web
- CLI
- Github Action

## Quickstart
```shell
$ cargo binstall freq-ai
$ freq-ai
```


## Origin
It was upon an evening not unlike any other that the toils of my labour grew so weighty in the chambers of my mind as to brook no further dismissal. I had set myself to the construction of a cloud of mine own devising, a work, I confess, of no small ambition, and found that its foundations demanded a most particular order of laying: first the accounts of users, then the permissions that govern them, then the documentation by which they are made intelligible, then the compliance by which they are made lawful, and at the last the security by which the whole is preserved from ruin. Each stone, you see, rested upon the one before it, and to misplace a single course was to invite the slow and silent decay of the edifice entire. The labour was not, in truth, beyond the compass of a single mind; yet it lay manifestly beyond the reach of a single pair of hands. And so, after the manner of those who, finding themselves outnumbered by their own undertakings, resolve to multiply their instruments rather than their hours, I set about the fashioning of further hands, and of a cycle by which they might turn in concert.


## CLI examples

```shell
# Launch the desktop UI (default subcommand)
$ freq-ai

# Review every open PR in the current repo
$ freq-ai code-review

# Work a single issue end-to-end (drafts a branch + PR)
$ freq-ai issue 42

# Address review threads on a PR
$ freq-ai fix-pr 1337

# Continuously work issues from a tracker issue
$ freq-ai loop 7

# Tidy the repo (stale branches, label drift, etc.)
$ freq-ai housekeeping

# Refresh AGENTS.md / project docs from the current state of the code
$ freq-ai refresh-docs

# Serve the web UI on http://localhost:3030
$ freq-ai serve

# Pick a different agent CLI on the fly
$ freq-ai --agent codex code-review
$ freq-ai --agent gemini issue 42
```

`--agent` accepts `claude`, `cline`, `codex`, `copilot`, `gemini`, `grok`, `junie`, `xai`, `cursor`. The matching CLI must be installed and authenticated. `--auto` skips confirmation prompts; `--dry-run` prints the resolved agent invocation without executing.

## Run it from CI: hands-off repo maintenance

Every CLI subcommand above is also available as a GitHub Action — [**geoffsee/freq-ai-action**](https://github.com/geoffsee/freq-ai-action). Wire it to `pull_request`, `issues`, or `schedule` and your repo starts maintaining itself: issues become PRs, PRs get reviewed, review threads get addressed, weekly housekeeping happens on its own.

A working end-to-end demo lives at [**geoffsee/freq-ai-hello-world**](https://github.com/geoffsee/freq-ai-hello-world) — a tiny Node project where labeling an issue `agent:work` is enough to land a merged PR with no further input.

```yaml
- uses: geoffsee/freq-ai-action@v1
  with:
    task: code-review
    agent: claude
  env:
    CLAUDE_CODE_OAUTH_TOKEN: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
```

## Status: Unstable (Active Development)
Expect unexpected breaking changes.

## Docs
- [Getting Started](docs/getting_started.md) — Installation, prerequisites, Desktop vs Web App, and CLI usage.
- [Workflow & Lifecycle](docs/workflow.md) — How an AI dev agent cycle works (Ideation to Retrospective), including Documentation Refresh actions.
- [Configuration & Setup](docs/configuration.md) — CLI options, bot account setup for Code Review, supported agents, and general tips.
- [Architecture](docs/architecture.md) — Project structure and internals.

## Contributing
- Open Issue -> Fork Repo -> Create Pull Request

## Contact
- File an issue for any questions or feedback.
