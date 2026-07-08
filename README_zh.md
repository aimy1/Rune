# ᚱ Rune (简体中文)

[![Rust Compile](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Startup](https://img.shields.io/badge/startup-%3C30ms-green.svg?style=flat-square)](#)

[English](./README.md) | [简体中文](./README_zh.md)

Rune 是一款基于 Rust 开发的现代化、极速终端启动器与命令面板（TUI Launcher & Command Palette）。受 VS Code 命令面板和 Raycast/Spotlight 等桌面启动器的启发，Rune 旨在成为 Linux 终端环境下以键盘驱动的中心化入口。

---

## ✨ 设计理念

- **极速响应 (<30ms)**：通过后台异步索引与本地文件缓存机制，实现瞬间冷启动，彻底告别磁盘 IO 带来的卡顿。
- **键盘优先**：零鼠标依赖，所有的交互与功能皆可通过直观的键盘快捷键触达。
- **极极简美学**：专业、沉静且无干扰的现代化暗色调界面（内置支持 Tokyo Night, Catppuccin, Nord, Gruvbox 和 Everforest 主题），拒绝花哨的 RGB 赛博朋克风。
- **插件化架构**：核心仅提供 TUI 渲染、模糊搜索和主题配置，所有的功能模块（包括应用、文件查找等系统功能）均作为插件实现。

---

## 🛠️ 功能矩阵

- 🖥️ **应用启动**：扫描系统 `.desktop` 文件，支持后台分离模式（Detached Session）启动图形或终端应用。
- 📁 **文件查找**：后台线程递归检索指定目录，支持对文本文件进行右侧实时预览，支持目录结构树渲染。
- 🐚 **命令执行**：输入任意 Shell 命令，支持后台静默执行或前台终端交互执行。
- 🧮 **计算器与单位换算**：实时解析数学表达式，支持长度、温度、质量等物理单位的双向换算。
- 🔑 **SSH 启动器**：解析 `~/.ssh/config` 配置，快速建立远程连接。
- 📋 **剪贴板历史**：自带剪贴板守护进程（Daemon），静默监听剪贴板变更，支持历史条目检索与复原。
- 🐳 **Docker & Systemd**：无需离开终端即可检索并启停容器（附带容器日志预览）与管理 systemd 系统服务。
- 🤖 **AI 助手**：深度融合 LLM 接口，支持 OpenAI, Gemini 以及本地 Ollama 模型的流式答复预览。
- 🔌 **外部脚本插件**：支持以任何语言（Bash, Python, Node, Go 等）编写脚本作为插件放置在配置目录下，实现无限扩展。

---

## 🚀 快速上手

### 前置要求
确保系统已安装 Rust 编译器与工具链：
```bash
cargo --version
```

### 安装构建
```bash
# 编译 Release 版本
cargo build --release

# 运行 Rune
./target/release/rune
```

### 剪贴板守护进程 (可选)
为了实现剪贴板历史的静默采集，可以在桌面系统启动项中以后台守护进程模式启动 Rune：
```bash
./target/release/rune --daemon &
```

---

## ⚙️ 配置文件

Rune 的配置文件存放于 `~/.config/rune/config.toml`。首次运行会自动生成默认配置：

```toml
[general]
shell = "bash"
editor = "nano"

[theme]
# 可选的内置主题: "tokyo_night", "catppuccin", "nord", "gruvbox", "everforest"
# 或是指向自定义主题 TOML 文件的绝对路径
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

# 文件检索配置
files_paths = ["~"]
files_ignore = [".git", "node_modules", "target", ".cache"]
files_max_depth = 4

# AI 接口配置
ai_provider = "ollama"  # 支持 "openai", "gemini", "ollama"
ai_api_key = ""
ai_model = "llama3"
ai_api_url = "http://localhost:11434/api/generate"
```

### 自定义主题

主题文件使用十六进制色彩进行配置：
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

## ⌨️ 快捷键

| 键位 | 描述 |
| :--- | :--- |
| `Char` / `Backspace` | 输入与修改搜索框文本 |
| `Up` / `Down` (or `Ctrl-p` / `Ctrl-n`) | 在匹配结果列表中上下移动选择项 |
| `Tab` / `Shift-Tab` | 切换当前激活的插件面板 |
| `Enter` | 触发执行选中项 of 默认动作 |
| `Esc` | 关闭退出 Rune |

---

## 🔌 自定义插件

通过在 `~/.config/rune/plugins/` 放入可执行脚本或二进制程序，即可轻松扩展 Rune。检索时，Rune 会把查询词作为第一个参数（`$1`）调用它，并读取其在 `stdout` 输出的 JSON 数组：

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

## 🏛️ 项目架构

Rune 采用严格的分层模块结构，确保项目的高可读性与高可扩展性：

```
src/
├── main.rs         # Entrypoint & CLI arguments parser / 入口与参数解析
├── core/
│   ├── app.rs      # Event coordinator & rendering loop / 事件循环与 TUI 协调器
│   └── plugin.rs   # Core traits and Context definitions / 插件接口及上下文规范
├── ui/
│   ├── draw.rs     # TUI view rendering with ratatui / 界面绘制实现
│   └── theme.rs    # Color theme loaders / 主题解析加载
├── plugins/        # Core plug-ins registry / 内置插件库 (Git, Docker, etc.)
├── search/         # Fuzzy matcher scoring / 模糊检索匹配
└── storage/        # History logs database & frecency ranker / 历史记录与 Frecency 权重计算器
```
