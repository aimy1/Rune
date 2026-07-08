# ᚱ Rune

[![Rust Compile](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Startup](https://img.shields.io/badge/startup-%3C30ms-green.svg?style=flat-square)](#)

[English](./README.md) | [简体中文](./README_zh.md)

Rune is a modern, blazing-fast Terminal User Interface (TUI) launcher and command palette for Linux, written in Rust. Inspired by VS Code's command palette and desktop launchers like Raycast, Rune acts as the central, keyboard-driven entry point for your terminal workspace.

---

## ✨ Design Philosophy

- **Instantaneous (<30ms)**: UI loads instantly using background indexing and caching to bypass slow disk lookups.
- **Keyboard-First**: Zero mouse interactions required. Everything is mapped to natural keyboard bindings.
- **Minimalist Aesthetic**: Professional, calm, and distraction-free dark modes (featuring Tokyo Night, Catppuccin, Nord, Gruvbox, and Everforest). No flashy RGB or fake terminal graphics.
- **Extensible Architecture**: Core contains only the TUI render loop, search matcher, and theme systems. All operational tools are isolated modules.

---

## 🛠️ Feature Matrix

- 🖥️ **Applications**: Scrapes system `.desktop` files and launches applications in detached background sessions.
- 📁 **Files**: Walks configured directories in a background worker and features directory layout listing or text file previews.
- 🐚 **Commands**: Executes raw commands, either in the background or interactive foreground terminal.
- 🧮 **Calculator & Conversions**: Dynamically evaluates math expressions and unit conversions (mass, temperature, length).
- 🔑 **SSH Hosts**: Parses Host configs from `~/.ssh/config` for instant TUI connections.
- 📋 **Clipboard Manager**: Runs an optional background history collector daemon that archives clips and restores them via search selection.
- 🐳 **Docker & Systemd**: Lists container statuses (plus log previews) and system services, with options to start/restart.
- 🤖 **AI Assistant**: Streamlined LLM API call interfaces (supports OpenAI, Gemini, and local Ollama API endpoints).
- 🔌 **Dynamic Extensions**: Easily drop scripts in any language (Bash, Python, Node, Go) that output JSON results into the plugins directory.

---

## 🚀 Getting Started

### Prerequisites

Ensure you have the Rust compiler and toolchain installed:
```bash
cargo --version
```

### Installation

Clone the repository and build from source:
```bash
# Compile for release target
cargo build --release

# Run Rune
./target/release/rune
```

### Background Clipboard Collector (Optional)

To enable persistent clipboard history logging, launch Rune in daemon mode in the background (e.g. from your WM startup config):
```bash
./target/release/rune --daemon &
```

---

## ⚙️ Configuration

Configurations are stored in `~/.config/rune/config.toml`. It is generated with default options on first launch:

```toml
[general]
shell = "bash"
editor = "nano"

[theme]
# Active built-in theme: "tokyo_night", "catppuccin", "nord", "gruvbox", "everforest"
# Or provide the absolute path to a custom theme TOML file
active = "catppuccin"

[plugins]
applications = true
files = true
commands = true
calculator = true
unit_converter = true
ssh = true
clipboard = true
git = true
docker = false
systemd = false
ai = false

# File indexing config
files_paths = ["~"]
files_ignore = [".git", "node_modules", "target", ".cache"]
files_max_depth = 4

# AI integration details
ai_provider = "ollama"  # "openai", "gemini", "ollama"
ai_api_key = ""
ai_model = "llama3"
ai_api_url = "http://localhost:11434/api/generate"
```

### Custom Themes

Theme files map colors using standard hex codes:
```toml
background = "#1e1e2e"
foreground = "#cdd6f4"
accent = "#cba6f7"
border = "#585b70"
selection = "#313244"
warning = "#f9e2af"
success = "#a6e3a1"
error = "#f38ba8"
```

---

## ⌨️ Keybindings

| Key | Description |
| :--- | :--- |
| `Char` / `Backspace` | Type and edit active query |
| `Up` / `Down` (or `Ctrl-p` / `Ctrl-n`) | Navigate list selections |
| `Tab` / `Shift-Tab` | Shift modes / filter active plugin tabs |
| `Enter` | Launch selection action |
| `Esc` | Quit Rune |

---

## 🔌 Custom Plugins API

Create custom plugins by dropping executable scripts or binaries into `~/.config/rune/plugins/`. When searching, Rune runs the executable with the active query as the first argument (`$1`) and expects a JSON array on `stdout`:

```json
[
  {
    "id": "item_id",
    "title": "Display Title",
    "subtitle": "Display Subtitle (Optional)",
    "score": 100,
    "preview": "# Markdown details\nRenders on the right panel.",
    "execute_cmd": "command_to_run",
    "execute_args": ["arg1", "arg2"],
    "run_in_terminal": false
  }
]
```

---

## 🏛️ Project Architecture

Rune separates modules strictly to maintain a Clean Architecture layout:

```
src/
├── main.rs         # Entrypoint & CLI arguments parser
├── core/
│   ├── app.rs      # Event coordinator & rendering loop
│   └── plugin.rs   # Core traits and Context definitions
├── ui/
│   ├── draw.rs     # TUI view rendering with ratatui
│   └── theme.rs    # Color theme loaders
├── plugins/        # Core plug-ins registry (Git, Docker, etc.)
├── search/         # Fuzzy matcher scoring
└── storage/        # History logs database & frecency ranker
```
