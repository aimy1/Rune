<div align="center">
 
 # ᚱ Rune
 
 **基于 Rust 构建的极速 TUI 应用启动器与命令面板，专为 Linux 打造。**
 
 *灵感来源于 Raycast 与 VS Code 命令面板 — 完全键盘驱动。*
 
 <br/>
 
 [![Language: Rust](https://img.shields.io/badge/Language-Rust-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
 [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
 [![Platform: Linux](https://img.shields.io/badge/Platform-Linux-informational?logo=linux&logoColor=white)](https://github.com/aimy1/Rune)
 [![GitHub Stars](https://img.shields.io/github/stars/aimy1/Rune?style=social)](https://github.com/aimy1/Rune/stargazers)
 
 <br/>
 
 **🌐 语言 / Language**
 
 [**English**](README.md) · [**简体中文**](README_zh.md)
 
 </div>
 ---
 <img width="2346" height="1425" alt="屏幕截图_20260709_103943" src="https://github.com/user-attachments/assets/a52509f3-61ed-4ad2-bd4d-7e26e0feb294" />
 <img width="2397" height="1432" alt="屏幕截图_20260709_103741" src="https://github.com/user-attachments/assets/772dbc6d-b463-4e4a-bdc5-e31aa6d228bf" />
 
 ## 📖 项目简介
 
 Rune 是一款现代化、轻量级、全键盘驱动的 **TUI 启动器**，专为 Linux 终端环境设计。它作为统一的命令入口，涵盖应用启动、文件浏览、Shell 命令执行、AI 对话、服务管理与剪贴板历史管理，无需离开终端即可完成一切。
 
 ---
 
 ## 🚀 核心功能
 
 | 图标 | 功能说明 |
 | :---: | :--- |
 | 🖥️ | **应用启动器** — 扫描 `.desktop` 快捷方式，以后台分离会话一键启动 GUI 应用，或在编辑文本命令时自动挂起 Alternate 屏幕退出时恢复（如 `nvim`） |
 | 📁 | **响应式文件管理器** — Dolphin 灵感的双栏文件浏览器，支持自适应列、收藏夹侧边栏、路径栏、本地过滤器 |
 | 🖼️ | **图像预览渲染** — 使用半像素字符（`▄`/`▀`）与真彩色 RGB 渲染图标与图片（侧边预览栏同步支持） |
 | ⏱️ | **历史记录管理** — 支持完全可用的“最近文件”和“最近访问目录”虚拟历史记录列表文件夹 |
 | 📋 | **交互式弹窗叠层** — 悬浮操作右键菜单（`m`/`M`）、带文本光标的安全交互式新建/重命名输入弹窗，以及带红色警告的删除确认二次验证弹窗 |
 | 🧮 | **计算器与单位换算** — 实时解析数学表达式；支持质量、温度、长度双向换算（精确至小数点后 4 位） |
 | 🔑 | **SSH 快速连接** — 解析 `~/.ssh/config`，即时发起 SSH 会话 |
 | 🐳 | **Docker & Systemd 控制** — 实时查看容器（含日志预览）与系统服务状态，支持一键启动/重启 |
 | 🤖 | **内置 AI 助手** — 实时 LLM 对话；支持 OpenAI、Gemini API 与本地 Ollama 服务 |
 
 ---
 
 ## 📁 文件管理器控制面板 `F2`
 
 按下 **`F2`** 呼出双栏文件浏览器。焦点切换循环：侧边栏 ➔ 文件列表 ➔ 当前路径栏 ➔ 搜索过滤框。
 
 ### 📱 响应式自适应列
 - **窄屏模式 (< 55 列)**：仅显示 `名称` 和 `大小`。
 - **中等屏幕 (55–75 列)**：显示 `名称`、`大小` 和 `修改时间`。
 - **宽屏模式 (> 75 列)**：显示 `名称`、`大小`、`权限` 和 `修改时间`。
 
 ### ⌨️ 键盘快捷键 (文件管理器内全局可用)
 | 键位 | 功能 |
 | :--- | :--- |
 | `Tab` / `Shift-Tab` | 在各个面板之间循环切换焦点（侧边栏 ➔ 文件列表 ➔ 路径栏 ➔ 搜索框） |
 | `Backspace` | 返回上一级目录（或退出虚拟历史记录视图） |
 | `m` / `M` | 打开 / 关闭悬浮操作右键菜单 |
 | `Alt-c` / `Alt-x` | 复制 / 剪切当前选中的文件（被剪切的文件会被自动变暗淡化） |
 | `Alt-v` | 粘贴剪贴板中的文件（命名冲突时自动重命名并添加数值后缀） |
 | `Alt-d` / `Delete` | 打开红框警告的删除二次确认弹窗 |
 | `Alt-r` | 重命名文件/文件夹（打开带原名的交互式输入弹窗，带文本光标） |
 | `Alt-n` / `Alt-f` | 新建文件夹 / 文件（打开交互式输入弹窗，带文本光标） |
 | `Alt-h` | 切换显示/隐藏以 `.` 开头的隐藏文件 |
 | `F4` / `Alt-t` | 在当前目录下启动用户默认 Shell 终端 |
 | `Alt-o` | 在系统的图形化文件管理器中打开当前目录 |
 
 ---
 
 ## ⚙️ 交互式设置中心 `F1`
 
 在 Rune 界面中随时按下 **`F1`** 唤起双栏系统设置面板。所有修改即时写入 `~/.config/rune/config.toml`，无需重启。
 
 | 设置项 | 详细说明 |
 | :---: | :--- |
 | 🎨 | **主题实时切换** — 在 Tokyo Night、Catppuccin、Nord、Gruvbox、Everforest 之间热切换 |
 | 🌐 | **双语界面切换** — 实时在**简体中文**与 **English** 之间切换，所有面板立即更新 |
 | 📝 | **文本编辑器绑定** — 绑定 `nano`、`vim`、`nvim`、`hx` 或自定义编辑器路径（带智能单选框逻辑） |
 | 🔌 | **插件独立开关** — 对 14 个内置插件逐一启用/禁用（包括 `file_manager` 等），标签页实时重建 |
 | 🤖 | **AI 提供商选择** — 切换 `openai`、`gemini`、`ollama`；端点与模型名称自动填充 |
 | 🔑 | **参数行内编辑** — 在底栏状态行直接手写 API Key、模型名称与 Endpoint URL（自动掩码保护） |
 | 📂 | **文件搜索深度** — 设置目录递归层数（2–6 层），保存时自动清空旧缓存并重建索引 |
 | ℹ | **关于 Rune** — 查看版本号、许可证、GitHub 链接与作者署名（焦点锁定左侧不进入右侧） |
 
 ---
 
 ## 🛠️ 安装方式
 
 > **前提条件**：已安装 [Rust 工具链](https://rustup.rs/) 与 Git。
 
 ### ⚡ 一键安装
 
 以下命令将自动完成：克隆源码 → 编译 → 安装二进制文件 → 注册剪贴板守护服务 → 清理缓存：
 
 ```bash
 git clone https://github.com/aimy1/Rune.git ~/.cache/rune_src && cd ~/.cache/rune_src && make install-user && make install-daemon && rm -rf ~/.cache/rune_src
 ```
 
 ### 🔧 手动安装
 
 ```bash
 # 1. 克隆仓库
 git clone https://github.com/aimy1/Rune.git && cd Rune
 
 # 2. 编译并安装至 ~/.local/bin/rune（无需 root 权限）
 make install-user
 
 # 3. 注册剪贴板守护进程为 systemd 用户服务（optional）
 make install-daemon
 ```
 
 > 请确保 `~/.local/bin` 已加入系统环境变量 `PATH`，随后直接运行：
 > ```bash
 > rune
 > ```
 
 ---
 
 ## ⌨️ 快捷键说明 (主启动器窗口)
 
 | 键位 | 功能 |
 | :--- | :--- |
 | `Char` / `Backspace` | 输入 / 修改搜索查询 |
 | `↑` / `↓` &nbsp;·&nbsp; `Ctrl-p/n` &nbsp;·&nbsp; `Ctrl-k/j` | 在结果列表中上下导航 |
 | `Tab` / `Shift-Tab` | 左右切换当前激活的插件分类标签页（切换模式） |
 | `Shift-↑` / `Shift-↓` &nbsp;·&nbsp; `Alt-j/k` &nbsp;·&nbsp; `PgUp/PgDn` | 滚动预览区域长文本 |
 | `Enter` | 执行选中项的默认动作 |
 | `F1` | 打开 / 关闭设置面板 |
 | `Esc` | 关闭当前叠层 / 退出程序 |
 
 ---
 
 ## 🔌 编写自定义插件
 
 将任意可执行脚本或二进制文件放入 `~/.config/rune/plugins/` 目录。Rune 将当前查询词作为第一个参数（`$1`）调用它，并读取其在 `stdout` 输出 of JSON 数组：
 
 ```json
 [
   {
     "id": "my_plugin_result",
     "title": "结果标题",
     "subtitle": "可选的副标题说明",
     "score": 100,
     "preview": "# Markdown 预览\n内容将渲染在右侧详情面板中。",
     "execute_cmd": "some-command",
     "execute_args": ["arg1", "arg2"],
     "run_in_terminal": false
   }
 ]
 ```
 
 ---
 
 ## 🏛️ 项目目录结构
 
 ```
 src/
 ├── main.rs          # 程序入口与命令行参数解析
 ├── core/
 │   ├── app.rs       # 主事件循环与状态协调器
 │   └── plugin.rs    # 插件接口 Trait 与上下文定义
 ├── ui/
 │   ├── draw.rs      # TUI 界面布局与渲染（ratatui）
 │   └── theme.rs     # 主题加载与调色板系统
 ├── plugins/         # 内置插件库（应用、Docker、AI 等）
 ├── search/          # 模糊匹配评分引擎
 └── storage/         # 缓存管理器与 Frecency 权重计算
 ```
 
 ---
 
 ## 📄 开源许可
 
 本项目遵循 **MIT License** 开源协议 — 详见 [LICENSE](LICENSE)。
 
 ---
 
 <div align="center">
 
 用 ❤️ 构建，作者 **aisaniya** · [GitHub](https://github.com/aimy1/Rune)
 
 </div>
