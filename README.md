# MMYCodeSwitch-API

<div align="center">

**Claude Code API 提供商一键切换工具**

[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app/)
[![Vue.js](https://img.shields.io/badge/Vue.js-3.x-green?logo=vue.js)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows-blue?logo=windows)]()

[功能介绍](#功能介绍) · [快速开始](#快速开始) · [打包发布](#打包发布) · [开发指南](#开发指南)

</div>

---

## ✨ 功能介绍

MMYCodeSwitch-API 是一款基于 **Tauri 2.x + Vue 3 + TypeScript** 构建的桌面工具，用于管理 Claude Code 的 API 提供商配置，实现：

- 🔑 **API Key 管理** — 添加、编辑、删除多个 API 提供商的密钥
- 🔄 **一键切换** — 在不同 API 提供商之间快速切换，无需手动修改配置文件
- 📋 **配置导入导出** — 备份和恢复所有提供商配置
- 🎨 **现代化界面** — 简洁直观的 Vue 3 组件化 UI
- ⚡ **轻量高效** — 基于 Tauri 内核，内存占用极低，启动速度快

### 适用场景

| 场景 | 说明 |
|------|------|
| 多账号管理 | 同时管理多个 Claude Code API 账号 |
| 快速切换 | 在不同服务商/密钥间一键切换 |
| 团队共享 | 通过导入/导出功能分享配置模板 |

---

## 🚀 快速开始

### 环境要求

> **注意：** 当前仅支持 **Windows** 平台。

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

### 方式一：一键打包脚本（推荐）

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

### 打包类型对比

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

双击 `.exe` 文件即可运行安装程序。

### 方式二：手动命令行

```powershell
# Release 构建
npm run tauri build

# Debug 构建
npm run tauri build --debug
```

### 打包产物说明

| 文件类型 | 格式 | 说明 | 大小参考 |
|----------|------|------|----------|
| NSIS | `.exe` | 单文件安装包，推荐分发 | ~8-12 MB |
| MSI | `.msi` | Windows Installer 格式 | ~10-15 MB |

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
├── build.ps1               # 一键打包脚本 (PowerShell)
├── build.bat               # 双击打包入口 (调用 build.ps1)
```

### 可用命令

| 命令 | 说明 |
|------|------|
| `npm install` | 安装前端依赖 |
| `npm run tauri dev` | 启动开发服务器（热重载） |
| `npm run tauri build` | 生产环境构建 |
| `.\build.ps1` | 一键打包（推荐） |

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
  Made with ❤️ by <a href="https://github.com/mmy">MMY</a>
</p>
