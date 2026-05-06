# Getting Started

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/geoffsee/freq-ai/master/install.sh | bash
```

This detects your OS and architecture, downloads the latest release binary, and installs it to `~/.local/bin`. Override the install directory with `FREQ_AI_INSTALL_DIR`:

```sh
FREQ_AI_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/geoffsee/freq-ai/master/install.sh | bash
```

Pre-built binaries are available for:
- Linux x86_64 / aarch64
- macOS aarch64 (Apple Silicon)
- Windows x86_64

### Prerequisites

- `gh` CLI authenticated (`gh auth login`)
- An AI agent on PATH (`claude`, `cline`, `codex`, `copilot`, `cursor`, `gemini`, `grok`, `junie`, or `xai`)

## Quick Start

You can choose between a native Desktop App, a Web Server App, or use the Command Line Interface.

### Desktop App

To launch the native desktop GUI:

```sh
freq-ai gui
```

*(Note: `freq-ai` defaults to `gui` if no subcommand is provided).*

The desktop UI has a sidebar for configuration, workflow actions, tracker
issues, open issues, and PRs. The editor panel includes tabs for agent output,
file browsing, security findings, interviews, chat, and **Personas**. Use the
Personas tab to create, edit, delete, or generate persistent user personas from
natural-language notes. Persona JSON files are saved beside the configured
`user_personas` skill file, so UXR workflows consume the same persona set you
manage in the UI.

### Web App

If you've installed `freq-ai` locally, you can serve the embedded Dioxus web app over HTTP. The required web assets are bundled inside the native binary.

```sh
freq-ai serve
```

By default, it will be available at `http://127.0.0.1:8080`. You can specify a custom port:

```sh
freq-ai serve --port 3000
```

### Development Scripts (Web vs Desktop Mode)

If you're working directly from the repository source code, `scripts/agent.sh` provides a convenient launch wrapper:

- **Web Mode (Default):** Run the script without any arguments to compile and serve the web app dynamically via the `dx` CLI.
  ```bash
  ./scripts/agent.sh
  ```

- **Desktop Mode:** Pass the `--desktop` flag to compile and launch the native desktop GUI via Cargo.
  ```bash
  ./scripts/agent.sh --desktop
  ```

### CLI Usage

Everything runs from the sidebar in the GUI, but you can also invoke specific workflows or actions from the CLI:

```sh
# Run sprint planning draft
freq-ai sprint-planning

# Fix review comments on a PR
freq-ai fix-pr 123

# Run a specific issue
freq-ai issue 45

# List presets or inspect one preset's workflows
freq-ai presets
freq-ai presets ux
```
