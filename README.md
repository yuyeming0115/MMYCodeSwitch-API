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
- 📁 **多项目管理** — 每个项目独立配置目录，互不干扰
- 📋 **配置导入导出** — 备份和恢复所有提供商配置
- 🗃️ **会话历史归档** — 切换供应商时自动归档历史记录
- 🎨 **现代化界面** — 简洁直观的 Vue 3 组件化 UI
- ⚡ **轻量高效** — 基于 Tauri 内核，内存占用极低，启动速度快

### 适用场景

| 场景 | 说明 |
|------|------|
| 多账号管理 | 同时管理多个 Claude Code API 账号 |
| 快速切换 | 在不同服务商/密钥间一键切换 |
| 多项目并行 | 不同项目使用不同 API 配置，完全隔离 |
| 开发调试追踪 | MVP→查bug→性能分析，各阶段使用不同模型 |
| 团队共享 | 通过导入/导出功能分享配置模板 |

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

#### 方式一：一键打包脚本（推荐）

**💻 Terminal 命令行：**
```bash
# DMG 安装包（推荐）— 双击打开，拖拽到 Applications
./build.sh

# 便携版 — .app 应用包，直接运行
./build.sh --portable

# 清理后重新构建
./build.sh --clean
```

#### 打包类型对比（macOS）

| 类型 | 命令 | 说明 | 文件大小 |
|------|------|------|---------|
| **DMG** ✨ | 默认 | 标准安装包，拖拽安装 | ~10-15 MB |
| **App** | `--portable` | .app 应用包，直接运行 | ~8-12 MB |

**输出位置：**
```
src-tauri/target/release/bundle/
├── dmg/           # DMG 安装包
│   └── MMYCodeSwitch-API_1.0.0_x64.dmg
└── macos/         # .app 应用包
    └── MMYCodeSwitch-API.app
```

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

### 打包产物总览

| 平台 | 格式 | 说明 | 大小参考 |
|------|------|------|----------|
| Windows | `.exe` (NSIS) | 单文件安装包 | ~8-12 MB |
| Windows | `.msi` | Windows Installer | ~10-15 MB |
| macOS | `.dmg` | 标准安装包 | ~10-15 MB |
| macOS | `.app` | 应用包（便携） | ~8-12 MB |
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
├── projects/            # 项目专属配置目录（方案C架构）
│   ├── {project_hash}/  # 项目路径 MD5 哈希
│   │   ├── settings.json      # 当前 API 配置
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
| 完全隔离 | 每个项目有独立配置目录，互不影响 |
| 全局供应商池 | 所有供应商配置集中管理，一处修改全局生效 |
| 会话归档 | 每次切换供应商自动记录历史，便于回溯 |
| 路径无关 | 项目路径变化只需更新绑定关系 |

### 应用配置 (`src-tauri/tauri.conf.json`)

主要配置项：

| 配置项 | 当前值 | 说明 |
|--------|--------|------|
| identifier | `com.mmy.codeswitch` | 应用唯一标识符 |
| bundle.targets | `nsis, msi` | 打包目标格式 |
| window.title | `MMYCodeSwitch-API` | 窗口标题 |
| window.size | `900×600` | 默认窗口尺寸 |

---

## 📄 License

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/yuyeming0115">MMY</a>
</p>
