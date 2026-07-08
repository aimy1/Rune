# ᚱ Rune (简体中文)

Rune 是一款基于 Rust 开发的现代化、极速终端启动器与命令面板（TUI Launcher & Command Palette）。受 macOS 的 Raycast 和 VS Code 命令面板的启发，Rune 旨在成为 Linux 终端环境下键盘驱动的中心化入口，用于快速启动应用、查找文件、执行命令、与 AI 对话以及管理系统状态。

---

## 🚀 核心功能

- 🖥️ **应用启动**：扫描系统内 `.desktop` 快捷方式，支持以后台分离模式（Detached Session）一键启动 GUI 图形应用或终端程序。
- 📁 **模糊文件搜索**：后台工作线程递归检索指定目录。在搜索结果中点击文本/代码文件，会自动调用配置的默认编辑器在终端内直接打开编辑。
- 🖼️ **半像素图像渲染器**：在详情预览面板中，使用半像素字符（`▄`、`▀`）及真彩色 RGB 直接绘制高精度的软件徽标、图标和本地图片。
- 🐚 **命令执行器**：直接在输入框输入 Shell 命令，支持后台静默运行或在前台开启交互式终端执行。
- 🧮 **计算器与单位换算**：实时解析数学表达式，并提供长度、温度、质量等物理单位的双向换算，数值格式化精确至小数点后 4 位。
- 🔑 **SSH 快速连接**：读取 `~/.ssh/config` 下的主机配置，在选择后瞬间在终端内发起 SSH 会话连接。
- 📋 **剪贴板历史管理**：提供基于 Systemd 的剪贴板历史收集守护进程，静默抓取剪贴历史，支持通过列表检索并一键复原。
- 🐳 **Docker 与 Systemd 控制器**：实时查看容器（附带日志预览）与系统服务的运行状态，支持快速启动与重启。
- 🤖 **内置 AI 助手**：内置 AI 问答终端，支持 OpenAI、Gemini 接口协议，以及本地运行的 Ollama 服务接口。
- 🔌 **外部脚本插件**：支持以任意语言（Python, Bash, Node, Go）编写脚本作为插件放置在配置目录下，实现无限扩展。

---

## ⚙️ 交互式设置中心（按 F1 键）

在 Rune 界面中随时按下 **`F1`** 键，可唤起直观的双栏系统设置管理中心：

- **🎨 主题实时热切换**：在 Tokyo Night, Catppuccin, Nord, Gruvbox 和 Everforest 之间一键热切换主题，界面自动重绘，无需重启。
- **🌐 双语动态翻译**：支持 **中文 (zh)** 与 **英文 (en)** 的动态热切换，界面的所有面板、操作状态信息会立刻完成翻译。
- **📝 文本编辑器绑定**：绑定您偏好的终端编辑器（如 `nano`, `vim`, `nvim`, `hx` 或自定义路径）。搜索文本文件并确认时会自动唤起该编辑器。
- **🔌 功能插件开关**：对内置的 11 个插件实例进行独立插拔开启或禁用，立刻动态生成对应的顶栏模式标签。
- **🤖 AI 提供商选择**：切换 AI 助手提供商（`openai`, `gemini`, `ollama`），系统会自动重载并生成对应服务商的默认补全 URL 与默认模型。
- **🔑 参数行内编辑**：选定选项后直接在底栏状态行中，对 AI 密钥 API Key、自定义模型名称和 API Endpoint URL 等参数进行手写录入，并支持掩码掩护。
- **📂 文件搜索深度上限**：设置 Walkdir 扫描的目录层级（支持 2 至 6 层），点击保存时会自动清空旧缓存以重构文件索引。
- **ℹ️ 关于 Rune**：查看项目软件的版本号、核心许可协议、GitHub 链接以及作者署名（**aisaniya**）。

---

## 🛠️ 构建与编译安装

确保系统已安装 Rust 编译器与工具链，利用项目内置的 `Makefile` 即可完成一键编译部署：

```bash
# 1. 编译并安装至用户本地路径 ~/.local/bin/rune (无需 root 权限)
make install-user

# 2. 注册并运行剪贴板守护进程自启动服务
make install-daemon
```

请确保 `~/.local/bin` 已加入您的系统环境变量 `PATH` 中。接着即可直接在终端运行 `rune` 启动。

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

## 🔌 编写自定义插件

通过在 `~/.config/rune/plugins/` 放入可执行脚本或二进制程序，即可轻松扩展 Rune。检索时，Rune 会把查询词作为第一个参数（`$1`）调用它，并读取其在 `stdout` 输出的 JSON 数组：

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

## 🏛️ 项目分层架构

```
src/
├── main.rs         # 入口与参数解析
├── core/
│   ├── app.rs      # 事件循环与 TUI 协调器
│   └── plugin.rs   # 插件接口及上下文规范
├── ui/
│   ├── draw.rs     # 界面绘制实现 (ratatui)
│   └── theme.rs    # 主题解析加载
├── plugins/        # 内置插件库 (Git, Docker, 命令行等)
├── search/         # 模糊检索匹配
└── storage/        # 历史记录与 Frecency 权重计算器
```
