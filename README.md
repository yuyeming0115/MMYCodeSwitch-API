# MMYCodeSwitch-API

<div align="center">

**Claude Code API 提供商一键切换工具**

[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app/)
[![Vue.js](https://img.shields.io/badge/Vue.js-3.x-green?logo=vue.js)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)]()

[功能介绍](#功能介绍) · [快速开始](#快速开始) · [打包发布](#打包发布) · [开发指南](#开发指南)

</div>

---

## ✨ 功能介绍

MMYCodeSwitch-API 是一款基于 **Tauri 2.x + Vue 3 + TypeScript** 构建的桌面工具，用于管理 Claude Code 的 API 提供商配置，实现：

- 🔑 **API Key 管理** — 添加、编辑、删除多个 API 提供商的密钥
- 🔄 **一键切换** — 在不同 API 提供商之间快速切换，无需手动修改配置文件
- 📁 **多项目管理** — 每个项目独立配置，自动注入到项目 `.claude/settings.json`
- 📋 **配置导入导出** — 备份和恢复所有提供商配置（支持密码保护）
- 🗃️ **会话历史归档** — 切换供应商时自动归档历史记录
- 📝 **开发模板管理** — CLAUDE.md 模板库，一键注入开发规范
- 🔧 **Skill 模板库** — Claude Code Skill 模板管理
- 🎨 **内置供应商模板** — 预置 7 大国产 AI 供应商模板，一键填写配置
- 🔗 **官方文档直达** — 每个模板附带帮助链接，快速查看最新模型列表
- ⚡ **轻量高效** — 基于 Tauri 内核，内存占用极低，启动速度快
- 🖥️ **系统托盘** — 最小化到托盘，右键快速切换供应商

### 适用场景

| 场景 | 说明 |
|------|------|
| 多账号管理 | 同时管理多个 Claude Code API 账号 |
| 快速切换 | 在不同服务商/密钥间一键切换 |
| 多项目并行 | 不同项目使用不同 API 配置，完全隔离 |
| 开发调试追踪 | MVP→查bug→性能分析，各阶段使用不同模型 |
| 团队共享 | 通过导入/导出功能分享配置模板 |

### 内置供应商模板

点击「添加供应商」即可选择预置模板，自动填写 API 地址和模型列表：

| 供应商 | API 地址 | 支持模型 | 特点 |
|--------|----------|----------|------|
| **阿里云百炼** | `dashscope.aliyuncs.com` | qwen-max, qwen-plus, deepseek-v3/r1 | 多模型聚合平台 ⭐ 推荐 |
| **DeepSeek** | `api.deepseek.com` | deepseek-chat, deepseek-coder, deepseek-reasoner | 推理增强 🆕 NEW |
| **智谱 GLM** | `open.bigmodel.cn` | glm-4-plus, glm-4-flash, glm-4-long | 国产领先 |
| **Kimi 月之暗面** | `api.moonshot.cn` | moonshot-v1-8k/32k/128k | 长文本专家 |
| **MiniMax** | `api.minimax.chat` | abab6.5-chat, abab6.5s-chat | 海螺 AI |
| **火山引擎 豆包** | `ark.cn-beijing.volces.com` | doubao-pro/lite-32k/128k | 字节跳动 |
| **腾讯混元** | `api.hunyuan.cloud.tencent.com` | hunyuan-lite/standard/pro/turbo | 腾讯云 |

> 💡 每个模板都附带官方文档链接，方便查看最新模型列表和 API 说明

---

## 🚀 快速开始

### 环境要求

| 依赖 | 最低版本 | 推荐版本 | 检查命令 |
|------|---------|---------|----------|
| Node.js | ≥18.0 | ≥20 LTS | `node --version` |
| npm | ≥9.0 | ≥10.x | `npm --version` |
| Rust | ≥1.70 | Latest Stable | `rustc --version` |
| Tauri CLI | ≥2.0 | 最新版 | `tauri --version` |

### 一键启动（开发模式）

```powershell
# 克隆项目
git clone https://github.com/mmy/MMYCodeSwitch-API.git
cd MMYCodeSwitch-API

# 安装依赖
npm install

# 开发模式运行（带热重载）
npm run tauri dev
```

开发模式下，修改前端代码会自动刷新，修改 Rust 后端代码会自动重新编译。

---

## 📦 打包发布

### Windows 平台

#### 方式一：一键打包脚本（推荐）

使用项目内置的打包脚本，自动完成全流程构建：

**🖱️ Windows 双击运行（最简单）：**
```
双击 build.bat → 选择模式
```

**💻 PowerShell 命令行：**
```powershell
# 便携版（推荐）— 直接运行，无需安装
.\build.ps1 -Portable

# 安装版 — 标准 NSIS 安装程序
.\build.ps1

# 单文件 EXE — 一个 exe 搞定（需要安装 7-Zip）
.\build.ps1 -SingleFile

# 清理后重新构建
.\build.ps1 -Portable -Clean
```

#### 打包类型对比（Windows）

| 类型 | 命令 | 说明 | 文件大小 |
|------|------|------|---------|
| **便携版** ✨ | `-Portable` | 双击即用，免安装，推荐分发 | ~8-10 MB |
| **安装版** | 默认 | 标准 NSIS 安装程序 | ~8-12 MB |
| **单文件** | `-SingleFile` | 自解压单 exe（需 7-Zip） | ~6-8 MB |

**输出位置：**
```
src-tauri/target/release/bundle/
├── nsis/          # NSIS 单文件安装包 (.exe)
│   └── MMYCodeSwitch-API_1.0.0_x64-setup.exe
└── msi/           # MSI 安装包 (.msi)
    └── MMYCodeSwitch-API_1.0.0_x64_en-US.msi
```

---

### macOS 平台

> ⚠️ **重要**：macOS 默认终端是 **zsh**，脚本中使用了 bash 语法，必须用 `bash` 显式运行：
> ```bash
> # ✅ 正确写法
> bash ./build.sh --portable
>
> # ❌ 错误写法（会报 bad substitution 或 permission denied）
> ./build.sh --portable
> ```

#### 方式一：一键打包脚本（推荐）

**💻 Terminal 命令行：**
```bash
# 便携版（推荐）— .app 应用包，直接双击运行，免安装
bash ./build.sh --portable

# DMG 安装包 — 双击打开，拖拽到 Applications
bash ./build.sh

# 清理后重新构建
bash ./build.sh --portable --clean

# Debug 模式（编译更快，体积更大）
bash ./build.sh --portable --dev
```

#### 打包类型对比（macOS）

| 类型 | 命令 | 说明 | 文件大小 |
|------|------|------|---------|
| **.app 便携版** ✨ | `--portable` | .app 应用包，双击即用，推荐分发拷贝 | ~8-12 MB |
| **DMG 安装包** | 默认 | 标准安装包，拖拽安装到 Applications | ~10-15 MB |

#### 常见问题（macOS）

| 问题 | 原因 | 解决方法 |
|------|------|----------|
| `permission denied: ./build.sh` | 脚本没有执行权限 | 运行 `chmod +x build.sh`，或直接用 `bash ./build.sh` |
| `bad substitution` | zsh 不兼容 `${VAR^^}` 语法 | 用 **`bash ./build.sh`** 替代 `./build.sh` |
| 找不到 `.app` | 旧版脚本用了 `--no-bundle` | 已修复，新版会自动生成完整 `.app` bundle |

**输出位置：**
```
项目根目录/
└── MMYCodeSwitch-API-Portable/          ← 便携版输出目录
    └── MMYCodeSwitch-API.app           ← 双击即可运行 ✅
```
拷贝整个 `MMYCodeSwitch-API-Portable` 文件夹到其他 Mac 即可使用。

---

### Linux 平台

```bash
# AppImage（推荐）— 单文件，直接运行
./build.sh

# Deb 包 — Debian/Ubuntu 系统安装包
./build.sh
```

**输出位置：**
```
src-tauri/target/release/bundle/
├── appimage/      # AppImage 单文件
│   └── MMYCodeSwitch-API_1.0.0_x64.AppImage
└── deb/           # Debian 包
    └── MMYCodeSwitch-API_1.0.0_amd64.deb
```

---

### 方式二：手动命令行（跨平台）

```bash
# Release 构建（自动根据平台生成对应格式）
npm run tauri build

# Debug 构建
npm run tauri build -- --debug
```
> macOS 用户同样可直接使用此命令（无需 bash 前缀），因为这是 npm 调用。

### 打包产物总览

| 平台 | 格式 | 说明 | 大小参考 |
|------|------|------|----------|
| Windows | `.exe` (NSIS) | 单文件安装包 | ~8-12 MB |
| Windows | `.msi` | Windows Installer | ~10-15 MB |
| macOS | `.dmg` | 标准安装包 | ~10-15 MB |
| macOS | **`.app` 文件夹** | 便携版（推荐） | ~8-12 MB |
| Linux | `.AppImage` | 单文件便携版 | ~10-15 MB |
| Linux | `.deb` | Debian/Ubuntu 包 | ~8-12 MB |

---

## 🛠️ 开发指南

### 项目结构

```
MMYCodeSwitch-API/
├── src/                    # Vue 前端源码
│   ├── components/         # Vue 组件
│   ├── App.vue             # 根组件
│   └── main.ts             # 入口文件
├── src-tauri/              # Tauri 后端 (Rust)
│   ├── src/lib.rs          # 主逻辑
│   ├── Cargo.toml          # Rust 依赖
│   ├── tauri.conf.json     # Tauri 配置
│   └── icons/              # 应用图标
├── public/                 # 静态资源
├── index.html              # HTML 入口
├── package.json            # 前端依赖
├── vite.config.ts          # Vite 构建配置
├── build.ps1               # 一键打包脚本 (Windows PowerShell)
├── build.bat               # 双击打包入口 (Windows)
├── build.sh                # 一键打包脚本 (Mac/Linux Bash)
```

### 可用命令

| 命令 | 说明 |
|------|------|
| `npm install` | 安装前端依赖 |
| `npm run tauri dev` | 启动开发服务器（热重载） |
| `npm run tauri build` | 生产环境构建 |
| `.\build.ps1` | Windows 一键打包 |
| `./build.sh` | Mac/Linux 一键打包 |

### 技术栈

| 层级 | 技术 | 用途 |
|------|------|------|
| 前端框架 | Vue 3 + TypeScript | UI 渲染与状态管理 |
| 构建工具 | Vite | 前端打包与 HMR |
| 桌面框架 | Tauri 2.x | 系统集成与应用外壳 |
| 后端语言 | Rust | 高性能系统调用处理 |
| 样式方案 | CSS / Scoped Styles | 界面样式 |

---

## 📋 配置说明

### 数据目录结构

应用数据存储在 `~/.mmycs/` 目录（Windows: `C:\Users\{用户名}\.mmycs\`）：

```
~/.mmycs/
├── config.json          # 应用主配置
├── .key                 # 加密密钥（自动生成）
├── providers/           # 全局供应商池
│   ├── provider_xxx.json
│   └── ...
├── projects/            # 项目专属配置目录（历史归档）
│   ├── {project_hash}/  # 项目路径 MD5 哈希
│   │   ├── binding.json       # 绑定关系记录
│   │   └── sessions/          # 会话历史归档
│   │       ├── 20260421_120000_Claude.json
│   │       └── ...
│   └── ...
├── backups/             # 配置备份
├── logs/                # 操作日志
│   └── switch.log
└── icons/               # 自定义图标
```

### 多项目模式架构

采用**项目专属配置目录**方案，核心特性：

| 特性 | 说明 |
|------|------|
| 配置生效 | API 配置注入到 `{项目目录}/.claude/settings.json`，Claude Code CLI 自动读取 |
| 完全隔离 | 每个项目有独立配置，互不影响 |
| 全局供应商池 | 所有供应商配置集中管理，一处修改全局生效 |
| 会话归档 | 每次切换供应商自动记录历史到 `~/.mmycs/projects/{hash}/sessions/` |
| 路径无关 | 项目路径变化只需更新绑定关系 |

### 备份导出/导入

**导出备份：**
- 点击「设置」→「导出备份」
- 可选包含 CLAUDE.md 模板和 Skill 模板
- **无密码模式**（默认）：导出 `.mmycs` 二进制文件，仅限本机导入，无需记住密码
- **密码保护模式**：勾选「设置密码保护」，跨机器迁移时需要输入密码

**导入备份：**
- 点击「设置」→「导入备份」
- **同机导入**：自动识别，无需密码，一键导入
- **跨机器导入**：需要输入导出时设置的密码
- 导入后自动恢复：供应商配置、模板、Skill、应用偏好设置

**文件格式对比：**

| 格式 | 扩展名 | 特点 |
|------|--------|------|
| 完整备份 | `.mmycs` | 二进制文件，包含供应商+模板+Skill+配置 |

### CLAUDE.md 模板管理

点击「设置」→「开发规则模板」进入模板管理：
- 创建/编辑/删除 CLAUDE.md 模板
- 模板用于定义项目的开发规范、编码风格等
- 模板存储在 `~/.mmycs/templates/` 目录

### Skill 模板库

点击「设置」→「Skill 模板库」进入 Skill 管理：
- 创建/编辑/删除 Skill 模板
- Skill 是 Claude Code 的扩展机制，用于自定义行为和能力
- Skill 存储在 `~/.mmycs/skills/` 目录

### 应用配置 (`src-tauri/tauri.conf.json`)

主要配置项：

| 配置项 | 当前值 | 说明 |
|--------|--------|------|
| identifier | `com.mmy.codeswitch` | 应用唯一标识符 |
| bundle.targets | `nsis, msi` | 打包目标格式 |
| window.title | `MMYCodeSwitch-API` | 窗口标题 |
| window.size | `510×620` | 默认窗口尺寸 |

---

## 📄 License

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/yuyeming0115">MMY</a>
</p>
