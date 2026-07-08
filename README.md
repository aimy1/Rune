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
- 🖼️ **Half-Block Image Renderer**: Renders high-quality pixel-art application logos and previews directly inside the details panel using half-block characters (`▄`, `▀`) and true RGB terminal colors.
- 🔌 **Dynamic Extensions**: Easily drop scripts in any language (Bash, Python, Node, Go) that output JSON results into the plugins directory.

---

## ⚙️ Interactive Settings Dashboard (Press F1)

Press **`F1`** inside Rune to open a professional, dual-pane Settings Control Center:

1. **🎨 TUI Theme Selector**: Live switch between Tokyo Night, Catppuccin, Nord, Gruvbox, and Everforest with instant UI redraws.
2. **🌐 Language Toggle**: Switch between **English (en)** and **Chinese (zh)** dynamically translating all UI headers, status bars, and help texts immediately.
3. **📝 Default Text Editor**: Choose from `nano`, `vim`, `nvim`, `hx` or input a custom command. Text/code files selected in search are automatically opened inside the terminal using this editor.
4. **🔌 Active Plugins Manager**: Toggle any of the 11 built-in plugins on-the-fly, dynamically rebuilding tabs and result lists.
5. **🤖 AI Provider Selector**: Choose your LLM engine (`openai`, `gemini`, `ollama`) to automatically populate default API models and completions endpoints.
6. **🔑 Credentials Inline Editor**: Hand-write API Keys, Custom Models, and API URLs inside an interactive text editor field in the status bar (with mask protection).
7. **📂 Files Max Depth Control**: Adjust directory scanning depth (2 to 6 layers), invalidating old index caches on-the-fly.
8. **ℹ️ About Rune**: View version information, GitHub links, and author credits (**aisaniya**).

---

## 🚀 Installation & Setup

### Prerequisites

Ensure you have the Rust compiler and toolchain installed:
```bash
cargo --version
```

### Build & Installation

We provide a `Makefile` to simplify binary compilation and deployment:

```bash
# 1. Build and install locally to ~/.local/bin/rune (No root permissions required)
make install-user

# 2. Or build and install system-wide to /usr/local/bin/rune
sudo make install
```

Make sure `~/.local/bin` is in your system `PATH`. You can then run the launcher directly by typing `rune` in your terminal.

### Clipboard Daemon Autostart (Optional)

To automatically collect clipboard history in the background, register and launch the daemon collector as a Systemd user service:

```bash
# Compile, register, enable, and start rune-daemon.service
make install-daemon
```

To stop and uninstall the service later:
```bash
make uninstall-daemon
```

---

## ⌨️ Keybindings

| Key | Description |
| :--- | :--- |
| `Char` / `Backspace` | Type and edit active query |
| `Up` / `Down` (or `Ctrl-p`/`n` / `Ctrl-k`/`j`) | Navigate list selections |
| `Tab` / `Shift-Tab` | Shift modes / filter active plugin tabs |
| `Shift-Up` / `Shift-Down` (or `Alt-j`/`k`, `PageUp`/`Down`) | Scroll preview pane text |
| `Enter` | Launch selection action |
| `F1` / `Esc` | Toggle Settings panel / Close launcher |

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
