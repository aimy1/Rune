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
- **极简美学**：专业、沉静且无干扰的现代化暗色调界面（内置支持 Tokyo Night, Catppuccin, Nord, Gruvbox 和 Everforest 主题），拒绝花哨的 RGB 赛博朋克风。
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
- 🖼️ **半像素图像渲染器**：在详情预览面板中，使用半像素字符（`▄`、`▀`）及真彩色 RGB 直接绘制高精度的软件徽标、图标和本地图片。
- 🔌 **外部脚本插件**：支持以任何语言（Bash, Python, Node, Go 等）编写脚本作为插件放置在配置目录下，实现无限扩展。

---

## ⚙️ 交互式设置面板（按 F1 键）

在 Rune 界面中随时按下 **`F1`** 键，可唤起直观的双栏系统设置管理中心：

1. **🎨 TUI 主题选择**：在 Tokyo Night, Catppuccin, Nord, Gruvbox 和 Everforest 之间一键 live 实时热切换主题，无需重启。
2. **🌐 语言切换**：支持 **中文 (zh)** 与 **英文 (en)** 的动态热切换，界面的所有面板、页脚提示、操作状态信息会立刻完成翻译翻译。
3. **📝 默认文本编辑器**：可选 `nano`, `vim`, `nvim`, `hx` 或是手写自定义命令。在搜索结果中点击文本/代码文件时，会自动在终端内调用此编辑器打开。
4. **🔌 插件开关管理**：对内置的 11 个插件实例进行热插拔开启或禁用，立刻动态生成对应的顶栏模式标签。
5. **🤖 AI 提供商选择**：选择 AI chatbot 底层服务商（`openai`, `gemini`, `ollama`），系统会自动重载并生成对应服务商的默认补全 URL 与默认模型。
6. **🔑 行内文本编辑器**：选定选项后直接在底栏状态行中，对 AI 密钥 API Key、自定义模型名称和 API Endpoint URL 等参数进行手写录入，并支持掩码掩护。
7. **📂 文件搜索深度上限**：设置 Walkdir 扫描的目录层级（支持 2 至 6 层），点击保存时会自动清空旧缓存以重构文件索引。
8. **ℹ️ 关于 Rune**：查看项目软件的版本号、核心许可协议、GitHub 链接以及作者署名（**aisaniya**）。

---

## 🚀 安装与部署

### 前置要求

确保系统已安装 Rust 编译器与工具链：
```bash
cargo --version
```

### 编译与安装

我们提供了 `Makefile` 来简化二进制程序的编译与部署：

```bash
# 1. 编译并安装至用户本地路径 ~/.local/bin/rune (推荐，无需 root 权限)
make install-user

# 2. 或编译并安装至系统全局路径 /usr/local/bin/rune (需要 sudo 权限)
sudo make install
```

请确保 `~/.local/bin` 已加入您的系统环境变量 `PATH` 中。接着即可直接在终端运行 `rune` 启动。

### 剪贴板守护进程自启动 (可选)

为了在后台自动搜集与整理剪贴板历史记录，您可以将收集守护程序注册并启动为 Systemd 用户服务：

```bash
# 自动编译、创建服务、配置开机自启并启动守护服务 rune-daemon.service
make install-daemon
```

如需在后续停止并卸载该自启动服务：
```bash
make uninstall-daemon
```

---

## ⌨️ 快捷键说明

| 键位 | 描述 |
| :--- | :--- |
| `Char` / `Backspace` | 输入与修改搜索框文本 |
| `Up` / `Down` (或 `Ctrl-p`/`n`，`Ctrl-k`/`j`) | 在匹配结果列表中上下移动选择项 |
| `Tab` / `Shift-Tab` | 切换当前激活的插件面板（模式） |
| `Shift-Up` / `Shift-Down` (或 `Alt-j`/`k`, `PageUp`/`Down`) | 滚动详情预览区域的长文本内容 |
| `Enter` | 触发执行选中项的默认动作 |
| `F1` / `Esc` | 开关系统设置面板 / 退出程序 |

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
