# MMYCodeSwitch-API 项目分析报告

> **生成日期：** 2026-04-22
> **项目版本：** v0.1.0
> **分析范围：** 完整项目架构、技术实现、功能模块

---

## 一、项目概述

### 1.1 项目定位

MMYCodeSwitch-API 是一款基于 **Tauri 2.x + Vue 3 + TypeScript** 构建的桌面工具，用于管理 Claude Code CLI 的 API 提供商配置。核心目标是实现一键切换不同 API 供应商，无需手动编辑配置文件。

### 1.2 核心价值

| 特性 | 说明 |
|------|------|
| 🔑 API Key 管理 | 添加、编辑、删除多个 API 提供商密钥，支持 AES-256-GCM 加密存储 |
| 🔄 一键切换 | 在不同 API 提供商之间快速切换，自动注入配置 |
| 📁 多项目管理 | 每个项目独立配置目录，互不干扰 |
| 📋 配置导入导出 | 备份和恢复所有提供商配置（密码二次加密） |
| 🗃️ 会话历史归档 | 切换供应商时自动归档历史记录 |
| 🎨 现代化界面 | 基于 Naive UI 的简洁直观 Vue 3 组件化 UI |
| ⚡ 轻量高效 | 基于 Tauri 内核，安装包仅 ~8-10MB，内存占用极低 |

### 1.3 适用场景

- 多账号管理：同时管理多个 Claude Code API 账号
- 快速切换：在不同服务商/密钥间一键切换
- 多项目并行：不同项目使用不同 API 配置，完全隔离
- 开发调试追踪：MVP→查bug→性能分析，各阶段使用不同模型
- 团队共享：通过导入/导出功能分享配置模板

---

## 二、技术架构分析

### 2.1 技术栈概览

| 层级 | 技术 | 版本 | 用途 |
|------|------|------|------|
| **桌面框架** | Tauri | 2.x | 系统集成与应用外壳 |
| **前端框架** | Vue 3 | 3.5.13 | UI 渲染与状态管理 |
| **构建工具** | Vite | 6.0.3 | 前端打包与 HMR |
| **UI 组件库** | Naive UI | 2.40.0 | 组件化 UI 元素 |
| **状态管理** | Pinia | 2.2.0 | 全局状态存储 |
| **国际化** | Vue I18n | 9.14.0 | 中英双语支持 |
| **后端语言** | Rust | - | 高性能系统调用处理 |
| **加密算法** | AES-256-GCM | - | API Key 安全存储 |

### 2.2 项目目录结构

```
MMYCodeSwitch-API/
├── src/                          # Vue 前端源码
│   ├── components/               # Vue 组件
│   │   ├── AppContent.vue        # 主应用容器（页面路由、核心逻辑）
│   │   ├── ProviderGrid.vue      # 供应商网格展示
│   │   ├── ProviderForm.vue      # 添加/编辑供应商表单
│   │   ├── ProjectList.vue       # 已激活项目列表
│   │   ├── QuickSetup.vue        # 快速配置模板
│   │   └── Settings.vue          # 设置面板
│   ├── stores/
│   │   └ app.ts                  # Pinia 全局状态管理
│   ├── i18n/
│   │   ├── index.ts              # i18n 配置入口
│   │   ├── zh.ts                 # 中文翻译
│   │   └ en.ts                   # 英文翻译
│   ├── App.vue                   # 根组件
│   └ main.ts                     # 入口文件
│   └ assets/                     # 静态资源
│   └ vite-env.d.ts               # TypeScript 类型声明
│
├── src-tauri/                    # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── lib.rs                # 主逻辑（Tauri 命令、托盘菜单）
│   │   ├── config.rs             # 配置读写、数据结构定义
│   │   ├── crypto.rs             # AES-256-GCM 加密解密
│   │   ├── inject.rs             # settings.json 注入逻辑
│   │   └ main.rs                 # 入口文件
│   ├── Cargo.toml                # Rust 依赖配置
│   ├── tauri.conf.json           # Tauri 应用配置
│   ├── capabilities/             # Tauri 权限配置
│   ├── icons/                    # 应用图标资源
│   └ gen/                        # 自动生成的配置
│   └ target/                     # Rust 编译产物
│
├── public/                       # 静态资源（内置图标等）
├── dist/                         # Vite 构建产物
├── MMYCodeSwitch-API-Portable/   # 便携版构建产物
│
├── build.ps1                     # 一键打包脚本 (PowerShell)
├── build.bat                     # 双击打包入口
├── package.json                  # 前端依赖配置
├── vite.config.ts                # Vite 构建配置
├── tsconfig.json                 # TypeScript 配置
├── README.md                     # 项目说明文档
└
└── 开发文档/                     # 开发文档目录
    └── MMYCodeSwitch-API开发文档.md  # 详细开发文档
```

---

## 三、核心模块分析

### 3.1 前端模块 (Vue 3)

#### 3.1.1 AppContent.vue - 主应用容器

**职责：**
- 页面路由控制（main / quickSetup / settings / form）
- 供应商切换流程（文件夹选择 → 检测重复 → 确认 → 注入 → 启动CLI）
- 项目列表管理（移除项目、继续开发、一键清理）
- 深浅色模式切换与持久化

**核心流程（点击供应商卡片）：**
```
用户点击 Provider 卡片
    ↓
弹出文件夹选择对话框
    ↓
检测该路径是否已有绑定项目
    ↓ (已存在且不同 Provider)
弹出确认切换对话框
    ↓ (确认或不存在)
调用 inject_to_project API
    ↓
自动启动 Claude Code CLI 终端
    ↓
显示成功消息
```

#### 3.1.2 ProviderGrid.vue - 供应商网格

**职责：**
- 以网格卡片形式展示所有供应商
- 当前激活供应商高亮显示（✓ 标记）
- 右键菜单：编辑 / 测试连通 / 删除
- 图标解析（支持用户上传和内置图标）

**关键特性：**
- 图标优先级：用户上传图片 → 内置图标 → 文字缩写 fallback
- 深色模式自适应样式

#### 3.1.3 ProviderForm.vue - 供应商表单

**职责：**
- 添加/编辑供应商配置
- 支持 API 模式和官方账号登录模式
- 粘贴解析功能（智能识别 Base URL、API Key、模型列表）
- 图标上传（PNG/SVG）

**智能解析支持的格式：**
| 输入格式 | 识别规则 |
|----------|----------|
| JSON（含 ANTHROPIC_BASE_URL） | 递归深挖，支持嵌套结构 |
| export ANTHROPIC_BASE_URL=... | Shell 变量格式解析 |
| 纯 URL 行（https:// 开头） | 提取为 baseUrl 候选 |
| sk- 开头字符串 | 提取为 apiKey 候选 |

#### 3.1.4 ProjectList.vue - 项目列表

**职责：**
- 展示已激活的项目列表
- 显示供应商图标、项目名称、切换时间、路径
- 快捷操作：继续开发（启动 CLI）、移除项目

#### 3.1.5 stores/app.ts - 状态管理

**核心数据结构：**
```typescript
interface Provider {
  id: string
  name: string
  icon_fallback: string
  provider_type: string        // "api" | "login"
  base_url?: string
  api_key_encrypted?: string   // AES-256-GCM 加密后的 Key
  models?: Record<string, string>
  notes?: string
  icon_path?: string
  created_at: string
  updated_at: string
}

interface ActiveProject {
  id: string
  name: string
  project_path: string
  provider_id: string
  provider_name: string
  config_dir?: string          // 项目专属配置目录路径
  created_at: string
  updated_at: string
}
```

**核心方法：**
- `init()` - 初始化应用
- `injectToProject(projectPath, providerId)` - 注入 API 到项目
- `upsertProvider(input)` - 创建/更新供应商
- `getProjectSessions(projectPath)` - 获取会话归档

---

### 3.2 后端模块 (Rust)

#### 3.2.1 lib.rs - 主逻辑

**Tauri 命令列表：**

| 命令 | 功能 |
|------|------|
| `init_app` | 初始化应用目录和加密密钥 |
| `get_app_config` / `save_app_config` | 应用配置读写 |
| `get_providers` | 获取所有供应商列表 |
| `upsert_provider` | 创建/更新供应商（自动加密 API Key） |
| `delete_provider` | 删除供应商 |
| `inject_to_project` | 注入 API 到项目专属配置目录 |
| `get_active_projects` | 获取已激活项目列表 |
| `remove_active_project` | 移除项目绑定记录 |
| `get_project_sessions` | 获取项目会话归档 |
| `launch_terminal` | 启动 Claude Code CLI 终端 |
| `parse_paste` | 智能解析粘贴内容 |
| `export_providers` / `import_providers` | 配置导入导出（密码二次加密） |
| `fetch_models` | 获取模型列表（后端代理，避开 CORS） |
| `test_provider` | 供应商连通性测试 |
| `detect_instances` | 自动检测 Claude Code 实例 |
| `check_active_claude_processes` | 检测活跃的 Claude CLI 进程 |

**系统托盘功能：**
- 快速切换供应商（子菜单）
- 显示/隐藏主窗口
- 退出应用

#### 3.2.2 config.rs - 配置管理

**数据存储位置：** `~/.mmycs/`

```
~/.mmycs/
├── config.json                  # 应用主配置
├── .key                         # 加密密钥（自动生成）
├── providers/                   # 全局供应商池
│   ├── provider_xxx.json
│   └── ...
├── projects/                    # 项目专属配置目录
│   ├── {project_hash}/          # 项目路径 MD5 哈希
│   │   ├── settings.json        # 当前 API 配置
│   │   ├── binding.json         # 绑定关系记录
│   │   └── sessions/            # 会话历史归档
│   │       ├── 20260421_120000_Claude.json
│   │       └── ...
│   └── ...
├── backups/                     # 配置备份
├── logs/                        # 操作日志
│   └── switch.log
└── icons/                       # 自定义图标
```

**关键函数：**
- `get_project_hash(project_path)` - 生成项目路径 MD5 哈希
- `ensure_project_config_dir(project_path)` - 确保项目专属目录存在
- `archive_session(project_path, provider, config_snapshot)` - 归档会话记录

#### 3.2.3 crypto.rs - 加密模块

**加密方案：** AES-256-GCM

```rust
pub fn encrypt(plaintext: &str, key_b64: &str) -> Result<String>
pub fn decrypt(ciphertext_b64: &str, key_b64: &str) -> Result<String>
pub fn generate_key() -> String    // 生成 32 字节随机密钥
```

**安全策略：**
- 首次运行时生成随机密钥，存储于 `~/.mmycs/.key`
- API Key 输入时明文显示，保存后加密存储
- 导出备份时用用户密码二次加密

#### 3.2.4 inject.rs - 配置注入

**注入字段列表：**
```rust
const ENV_KEYS: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_REASONING_MODEL",
];
```

**注入流程：**
```
读取当前 settings.json（不存在则创建空对象）
    ↓
备份现有文件到 ~/.mmycs/backups/
    ↓
清除所有 API 相关 env 字段（避免残留）
    ↓
写入新配置（API Key、Base URL、模型）
    ↓
记录切换日志
    ↓
归档会话记录（记录配置快照）
```

---

## 四、多项目模式架构

### 4.1 设计原则

采用**项目专属配置目录**方案，核心特性：

| 特性 | 说明 |
|------|------|
| 完全隔离 | 每个项目有独立配置目录，互不影响 |
| 全局供应商池 | 所有供应商配置集中管理，一处修改全局生效 |
| 会话归档 | 每次切换供应商自动记录历史，便于回溯 |
| 路径无关 | 项目路径变化只需更新绑定关系 |

### 4.2 工作流程

```
用户选择项目文件夹
    ↓
生成项目路径 MD5 哈希
    ↓
创建 ~/.mmycs/projects/{hash}/ 目录
    ↓
写入 settings.json（仅修改 env 字段）
    ↓
写入 binding.json（绑定关系）
    ↓
归档会话记录到 sessions/
    ↓
更新 active_projects 列表
    ↓
启动 Claude Code CLI
```

### 4.3 一键清理功能

检测逻辑：
1. 获取所有项目路径
2. 调用 PowerShell/ps 命令检测活跃的 Claude CLI 进程
3. 标记无活跃进程的项目为待清理
4. 弹出确认框，批量移除

---

## 五、构建与部署

### 5.1 打包脚本 (build.ps1)

**支持模式：**

| 类型 | 命令 | 说明 | 文件大小 |
|------|------|------|---------|
| **便携版** ✨ | `-Portable` | 双击即用，免安装 | ~8-10 MB |
| **安装版** | 默认 | NSIS 安装程序 | ~8-12 MB |
| **单文件** | `-SingleFile` | 自解压单 exe（需 7-Zip） | ~6-8 MB |

**构建流程：**
```
[1/6] 检查依赖（Node.js、Rust、可选 7z）
    ↓
[2/6] 清理旧构建产物（可选 -Clean）
    ↓
[3/6] 安装前端依赖（npm install）
    ↓
[4/6] 编译 Rust 后端（tauri build --no-bundle）
    ↓
[5/6] 创建输出包（便携版复制 DLLs，安装版运行 NSIS）
    ↓
[6/6] 显示构建结果
```

### 5.2 开发命令

| 命令 | 说明 |
|------|------|
| `npm install` | 安装前端依赖 |
| `npm run tauri dev` | 启动开发服务器（热重载） |
| `npm run tauri build` | 生产环境构建 |
| `.\build.ps1 -Portable` | 一键打包便携版 |

---

## 六、国际化支持

### 6.1 语言切换

- 支持中文（zh）和英文（en）
- 实时切换，无需重启
- 通过 Vue I18n 实现

### 6.2 翻译覆盖

主要功能均有完整翻译：
- 供应商管理（添加、编辑、删除、测试连通）
- 项目管理（注入、移除、继续开发）
- 设置面板（语言切换、导出导入）
- 错误提示和成功消息

---

## 七、UI/UX 设计

### 7.1 主界面布局

```
┌─────────────────────────────────────────────────┐
│  MMYCodeSwitch-API    [─][□][✕]                  │ ← 自定义标题栏
├─────────────────────────────────────────────────┤
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐           │
│  │[图标]│ │[图标]│ │[图标]│ │[图标]│           │ ← Provider 网格
│  │Claude│ │How88 │ │ 2API │ │ ALI  │           │
│  │  ✓   │ │      │ │      │ │      │           │
│  └──────┘ └──────┘ └──────┘ └──────┘           │
│  ┌──────┐                                       │
│  │  +   │                                       │ ← 添加按钮
│  └──────┘                                       │
├─────────────────────────────────────────────────┤
│  📂 已打开的项目 (N)                             │ ← 项目列表标题
│  ┌─────────────────────────────────────────────┐│
│  │ [图标] 项目名称    Provider  刚刚    ▶ ✕   ││ ← 项目卡片
│  │        ...路径截断...                       ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  [⚡ 快速配置] [🌙] [⚙️]                         │ ← 工具栏
├─────────────────────────────────────────────────┤
│  右键卡片可编辑/删除        [🧹 一键清理]       │ ← 状态栏
└─────────────────────────────────────────────────┘
```

### 7.2 设计特点

- **无边框窗口：** 自定义标题栏，支持拖拽、最小化、最大化、关闭
- **深色模式：** 全局样式切换，组件自适应
- **响应式布局：** 供应商网格自动换行
- **自定义滚动条：** 细滚动条，透明背景

---

## 八、安全性分析

### 8.1 API Key 安全策略

| 阶段 | 处理方式 |
|------|----------|
| 输入时 | 明文显示（方便核对粘贴） |
| 保存后界面显示 | 星号遮盖（仅显示前4后4位） |
| 编辑时 | 输入框默认空，留空不更新 |
| 本地存储 | AES-256-GCM 加密 |
| 导出备份 | 用户自设备份密码二次加密 |
| 导入时 | 验证备份密码解密，重新加密存储 |

### 8.2 数据隔离

- 独立管理空间 `~/.mmycs/`，不污染 `.claude/` 目录
- 仅修改 `settings.json` 的 `env` 字段，其余字段原样保留
- 切换前自动备份现有配置

---

## 九、项目亮点与创新

### 9.1 技术创新

1. **多项目专属配置目录架构：** 每个项目独立配置，通过 MD5 哈希标识，实现完全隔离
2. **智能粘贴解析：** 递归 JSON 解析，支持嵌套结构、Shell export 格式、裸值识别
3. **会话历史归档：** 每次切换自动记录配置快照，便于回溯
4. **一键清理未活跃项目：** 自动检测 CLI 进程，批量清理

### 9.2 用户体验优化

1. **一键切换流程：** 点击 → 选文件夹 → 自动注入 → 自动启动 CLI
2. **系统托盘快捷操作：** 右键菜单快速切换供应商
3. **深浅色模式切换：** 状态持久化，重启恢复
4. **中英双语实时切换：** 国际化支持

---

## 十、待优化与建议

### 10.1 功能扩展建议

| 建议 | 说明 |
|------|------|
| macOS/Linux 支持 | 当前仅支持 Windows，可扩展跨平台 |
| 云端同步 | 支持配置云端备份同步 |
| 模型价格展示 | 在供应商卡片显示模型价格信息 |
| 批量操作 | 批量导入/删除供应商 |

### 10.2 代码优化建议

| 建议 | 说明 |
|------|------|
| 单元测试 | 为 Rust 后端添加单元测试 |
| 错误处理 | 统一错误处理机制，优化用户提示 |
| 代码复用 | ProviderGrid 和 ProjectList 的图标解析逻辑可提取为公共函数 |

---

## 十一、总结

MMYCodeSwitch-API 是一款设计精良、功能完善的 Claude Code API 管理工具。项目采用 Tauri + Vue 3 技术栈，实现了轻量高效的桌面应用。核心的多项目专属配置目录架构创新性地解决了多项目并行开发的配置隔离问题，智能粘贴解析和一键清理功能显著提升了用户体验。

项目代码结构清晰，前后端分离明确，Rust 后端提供了高性能的系统调用处理，Vue 3 前端实现了现代化的组件化 UI。安全性方面，AES-256-GCM 加密保障了 API Key 的安全存储。

整体而言，项目已完成 MVP 阶段的核心功能，架构设计合理，具有良好的扩展性，为后续功能迭代奠定了坚实基础。

---

**报告结束**

*生成工具：Claude Code*
*分析日期：2026-04-22*