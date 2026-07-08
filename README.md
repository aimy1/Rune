<div align="center">

# ᚱ Rune

**A blazing-fast TUI application launcher & command palette for Linux, written in Rust.**

*Inspired by Raycast and VS Code's command palette — fully keyboard-driven.*

<br/>

[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux-informational?logo=linux&logoColor=white)](https://github.com/aimy1/Rune)
[![GitHub Stars](https://img.shields.io/github/stars/aimy1/Rune?style=social)](https://github.com/aimy1/Rune/stargazers)

<br/>

**🌐 Language / 语言**

[**English**](README.md) · [**简体中文**](README_zh.md)

</div>
![Uploading image.png…]()

---

## 📖 Overview

Rune is a modern, lightweight, keyboard-driven **TUI launcher** for Linux. It serves as a central command entry point — launch apps, find files, run shell commands, query AI, manage services, and browse clipboard history, all without leaving your terminal.

---

## 🚀 Core Features

| Feature | Description |
| :---: | :--- |
| 🖥️ | **App Launcher** — Index & search `.desktop` files; launch GUI apps in detached sessions |
| 📁 | **Fuzzy File Search** — Background recursive directory scan; click text files to open in your editor |
| 🖼️ | **Image Preview** — Renders icons & images via half-block characters (`▄`/`▀`) with true RGB colors |
| 🐚 | **Shell Executor** — Run commands silently in background or interactively in a new terminal |
| 🧮 | **Calculator & Unit Converter** — Evaluate expressions; convert mass, temperature, length (4 decimal places) |
| 🔑 | **SSH Launcher** — Parse `~/.ssh/config` and launch SSH sessions instantly |
| 📋 | **Clipboard Manager** — Systemd-based daemon captures clipboard history; restore entries from a searchable list |
| 🐳 | **Docker & Systemd** — Inspect containers (with log preview) and system services; start/restart with a keystroke |
| 🤖 | **AI Chatbot** — Real-time LLM chat; supports OpenAI, Gemini API, and local Ollama |
| 🔌 | **Script Plugins** — Drop any executable (Python, Bash, Node, Go) into the plugins dir to extend Rune |

---

## ⚙️ Settings Control Center `F1`

Press **`F1`** at any time to open the interactive dual-pane Settings dashboard. All changes are saved instantly to `~/.config/rune/config.toml`.

| Setting | Details |
| :---: | :--- |
| 🎨 | **Live Theme** — Cycle through Tokyo Night, Catppuccin, Nord, Gruvbox, Everforest |
| 🌐 | **Language** — Toggle interface between **English** and **简体中文** in real-time |
| 📝 | **Text Editor** — Bind `nano`, `vim`, `nvim`, `hx` or any custom editor path |
| 🔌 | **Plugin Toggles** — Enable/disable any of the 11 built-in plugins; tab bar rebuilds instantly |
| 🤖 | **AI Provider** — Switch between `openai`, `gemini`, `ollama`; endpoints & models auto-populate |
| 🔑 | **Credentials Editor** — Inline-edit API keys, models, and endpoint URLs in the status bar (masked) |
| 📂 | **File Search Depth** — Set directory traversal depth (2–6 levels); cache auto-flushes on save |
| ℹ️ | **About** — Version, license, GitHub repo, and author credits |

---

## 🛠️ Installation

> **Prerequisites**: [Rust toolchain](https://rustup.rs/) and Git.

### ⚡ One-Click Install

Clone, build, install the binary, register the clipboard daemon, and auto-clean up in one command:

```bash
git clone https://github.com/aimy1/Rune.git ~/.cache/rune_src && cd ~/.cache/rune_src && make install-user && make install-daemon && rm -rf ~/.cache/rune_src
```

### 🔧 Manual Install

```bash
# 1. Clone the repository
git clone https://github.com/aimy1/Rune.git && cd Rune

# 2. Build & install binary to ~/.local/bin/rune (no root required)
make install-user

# 3. Register clipboard daemon as a systemd user service (optional)
make install-daemon
```

> Make sure `~/.local/bin` is in your `PATH`, then simply run:
> ```bash
> rune
> ```

---

## ⌨️ Keybindings

| Key | Action |
| :--- | :--- |
| `Char` / `Backspace` | Type / edit search query |
| `↑` / `↓` &nbsp;·&nbsp; `Ctrl-p/n` &nbsp;·&nbsp; `Ctrl-k/j` | Navigate result list |
| `Tab` / `Shift-Tab` | Switch active plugin tab |
| `Shift-↑` / `Shift-↓` &nbsp;·&nbsp; `Alt-j/k` &nbsp;·&nbsp; `PgUp/PgDn` | Scroll preview pane |
| `Enter` | Execute primary action |
| `F1` | Open / close Settings dashboard |
| `Esc` | Close overlay / quit |

---

## 🔌 Writing Custom Plugins

Drop any executable script or binary into `~/.config/rune/plugins/`. Rune calls it with the current query as `$1` and reads a JSON array from `stdout`:

```json
[
  {
    "id": "my_plugin_result",
    "title": "Result Title",
    "subtitle": "Optional subtitle shown below",
    "score": 100,
    "preview": "# Markdown Preview\nRendered in the right-hand details pane.",
    "execute_cmd": "some-command",
    "execute_args": ["arg1", "arg2"],
    "run_in_terminal": false
  }
]
```

---

## 🏛️ Project Structure

```
src/
├── main.rs          # CLI entry point & argument parser
├── core/
│   ├── app.rs       # Main event loop & state coordinator
│   └── plugin.rs    # Plugin trait definitions & context
├── ui/
│   ├── draw.rs      # TUI layout & rendering (ratatui)
│   └── theme.rs     # Theme loading & palette system
├── plugins/         # Built-in plugin registry (Apps, Docker, AI, etc.)
├── search/          # Fuzzy match scoring engine
└── storage/         # Cache manager & frecency scoring
```

---

## 📄 License

This project is licensed under the **MIT License** — see [LICENSE](LICENSE) for details.

---

<div align="center">

Made with ❤️ by **aisaniya** · [GitHub](https://github.com/aimy1/Rune)

</div>
