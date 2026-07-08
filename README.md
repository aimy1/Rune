# ᚱ Rune

A modern, lightweight, and blazing-fast Terminal User Interface (TUI) application launcher and command palette for Linux, written in Rust. Inspired by macOS's Raycast and VS Code's command palette, Rune serves as a central, keyboard-driven entry point to launch apps, locate files, run commands, query AI, and configure your system workspace instantly.

---

## 🚀 Core Features

- 🖥️ **Desktop Applications**: Instantly index and search system `.desktop` files, launching GUI applications in detached background sessions.
- 📁 **Fuzzy File Search**: Recursively scan directory paths in a background thread. Clicking text files opens them directly inside your preferred terminal editor.
- 🖼️ **Half-Block Image Preview**: Renders high-quality pixel-art logo overlays and previews directly in the details panel using half-block characters (`▄`, `▀`) and true RGB terminal colors.
- 🐚 **Command Executor**: Enter system Shell commands for immediate detached background execution or interactive foreground terminal runs.
- 🧮 **Calculator & Unit Converter**: Dynamically evaluate mathematical expressions and convert physical units (mass, temperature, length) with 4-decimal precision.
- 🔑 **SSH Launcher**: Read config entries from `~/.ssh/config` for instant SSH connection launches.
- 📋 **Clipboard Manager**: Runs a Systemd-integrated clipboard monitor daemon to capture clips, letting you search and restore them from history.
- 🐳 **Docker & Systemd Services**: Live inspect containers (with log previews) and system services, with quick start/restart controls.
- 🤖 **AI Chatbot**: Talk to LLMs in real-time, supporting OpenAI, Gemini API, and local Ollama server configurations.
- 🔌 **Shell Script Plugins**: Extend Rune simply by dropping executable scripts (Python, Bash, Node, Go) that output JSON results into the plugins directory.

---

## ⚙️ Interactive TUI Control Center (F1)

Press **`F1`** inside Rune to open a dual-pane Settings Control Center. Changes are saved instantly to `~/.config/rune/config.toml`:

- **🎨 Live Theme Swapping**: Cycle through Tokyo Night, Catppuccin, Nord, Gruvbox, and Everforest with instant screen updates.
- **🌐 Bilingual Toggle**: Live switch interface texts between **English (en)** and **简体中文 (zh)**.
- **📝 Text Editor Routing**: Bind your default terminal editor (`nano`, `vim`, `nvim`, `hx` or custom paths). Text/code files selected in search are automatically opened inside the terminal using this editor.
- **🔌 Plugins Switchboard**: Toggle any of the 11 functional plugins to instantly rebuild the tabs bar and filter search results.
- **🤖 AI Provider presets**: Switch between `openai`, `gemini`, and `ollama` providers, auto-populating completions endpoints and model names.
- **🔑 Credentials Inline Editor**: Hand-write API Keys, Models, and Endpoint URLs directly in the bottom status bar with mask protection.
- **📂 File Index Depth**: Adjust directory traversal depths (from 2 to 6 layers), automatically flushing old cache files to force an immediate re-sweep.
- **ℹ️ About Project**: Structured credits page displaying license, repository details, and author name (**aisaniya**).

---

## 🛠️ Build & Installation

Ensure you have Rust and Cargo installed, then compile and install using the provided `Makefile`:

```bash
# 1. Build and install locally to ~/.local/bin/rune
make install-user

# 2. Add clipboard history manager user service (Autostart)
make install-daemon
```

Ensure `~/.local/bin` is added to your shell's `PATH` variable. Run Rune directly from your terminal by typing:
```bash
rune
```

---

## ⌨️ Keybindings

| Key | Description |
| :--- | :--- |
| `Char` / `Backspace` | Input query characters / clear text |
| `Up` / `Down` (or `Ctrl-p`/`n`, `Ctrl-k`/`j`) | Move selection cursor in result lists |
| `Tab` / `Shift-Tab` | Shift modes / filter search results by active plugin tabs |
| `Shift-Up` / `Shift-Down` (or `Alt-j`/`k`, `PageUp`/`Down`) | Scroll long text previews |
| `Enter` | Trigger primary action of selected result |
| `F1` / `Esc` | Toggle Settings dashboard / Quit launcher |

---

## 🔌 Writing Custom Plugins

Create custom plugins by dropping executable scripts or binaries into `~/.config/rune/plugins/`. Rune executes them with the current query as the first argument (`$1`) and expects a JSON array on `stdout`:

```json
[
  {
    "id": "unique_id",
    "title": "Display Title",
    "subtitle": "Optional Subtitle Description",
    "score": 100,
    "preview": "# Markdown Title\nThis text is rendered in the preview pane.",
    "execute_cmd": "command_to_execute",
    "execute_args": ["arg1", "arg2"],
    "run_in_terminal": false
  }
]
```

---

## 🏛️ Project Directory Structure

```
src/
├── main.rs         # CLI argument parser & main entrypoint
├── core/
│   ├── app.rs      # Event coordinator & rendering loop
│   └── plugin.rs   # Core traits and Context definitions
├── ui/
│   ├── draw.rs     # TUI view layout drawing (ratatui)
│   └── theme.rs    # Themes loading system
├── plugins/        # Core plug-ins registry (Git, Docker, etc.)
├── search/         # Fuzzy matcher scoring
└── storage/        # Cache manager & frecency scoring logs
```
