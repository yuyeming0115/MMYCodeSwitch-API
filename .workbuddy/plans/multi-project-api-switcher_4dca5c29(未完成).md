---
name: multi-project-api-switcher
overview: 将 MMYCodeSwitch-API 从「单实例+切换替换」模式重构为「多项目独立隔离」模式：点击 Provider 图标 → 弹出文件夹选择器 → 选择项目路径 → 注入该项目的 .claude/settings.json；同时新增已打开项目列表的管理界面（查看、移除、重新切换），支持重复选择时确认提示。
design:
  architecture:
    framework: vue
  styleKeywords:
    - Glassmorphism Dashboard
    - Clean Minimalist
    - Developer Tool Aesthetic
    - Green Accent Theme (#18a058)
    - Dark/Light Mode Support
    - Rounded Cards (14px)
    - Micro-interactions
  fontSystem:
    fontFamily: Inter, system-ui, -apple-system, sans-serif
    heading:
      size: 16px
      weight: 700
    subheading:
      size: 13px
      weight: 600
    body:
      size: 13px
      weight: 400
  colorSystem:
    primary:
      - "#18A058"
      - "#36AD6A"
      - "#0e7a3f"
    background:
      - "#F5F5F5"
      - "#FFFFFF"
      - "#FAFAFA"
    text:
      - "#333333"
      - "#666666"
      - "#888888"
    functional:
      - "#D03050"
      - "#E89834"
      - "#18A058"
todos:
  - id: backend-data-model
    content: 重构 Rust 后端数据模型：config.rs 新增 ActiveProject 结构体和 AppConfig.active_projects 字段，实现 load/save 辅助函数
    status: pending
  - id: backend-commands
    content: 新增 Rust Tauri Commands：inject_to_project、get_active_projects、remove_active_project，修改 lib.rs 注册命令并调整托盘菜单逻辑
    status: pending
    dependencies:
      - backend-data-model
  - id: frontend-store
    content: 重构 Pinia Store：app.ts 新增 ActiveProject 类型定义、activeProjects 状态、项目 CRUD 方法和加载逻辑
    status: pending
  - id: frontend-project-list
    content: 新建 ProjectList.vue 组件：已激活项目卡片列表，展示项目名/路径/Provider/时间，支持移除操作，深色模式适配
    status: pending
    dependencies:
      - frontend-store
  - id: frontend-main-flow
    content: 重写 AppContent.vue 主交互流程：doSwitch() 改为弹文件夹选择框→检测重复→确认→注入，集成 ProjectList 到主界面布局
    status: pending
    dependencies:
      - frontend-store
      - backend-commands
  - id: frontend-settings
    content: 精简 Settings.vue：将 Instance 多实例管理替换为「全局默认目录」单项配置
    status: pending
  - id: i18n-update
    content: 补充 i18n 国际化文案：zh.ts 和 en.ts 新增项目管理相关的约 15 个翻译 key
    status: pending
---

## Product Overview

MMYCodeSwitch-API 是一款 Tauri 桌面应用，用于管理多个 AI API 供应商配置并注入到 Claude Code 的 settings.json 中。当前版本只支持**单实例模式**（固定写入 `~/.claude/settings.json`），每次点击不同 Provider 都会覆盖同一个配置文件。本次改造的核心目标是实现**多项目管理模式**：用户点击某个 API 图标时，弹出一个文件夹选择框，选择一个项目目录，将 API 配置注入到该项目目录下的 `.claude/settings.json`，从而实现每个项目使用不同的 API 供应商，互不干扰。

## Core Features

- **动态项目文件夹选择**：点击 Provider 图标后，弹出系统文件夹选择对话框，让用户选择目标项目路径，而非使用固定的 `~/.claude` 目录
- **项目级配置隔离**：每个选中的项目目录下自动创建/更新 `.claude/settings.json`，各项目的 API 配置完全独立
- **已激活项目管理面板**：在主界面展示所有已绑定 API 的项目列表卡片（显示项目名称、使用的 Provider、绑定时间），支持移除项目绑定
- **重复项目检测与确认**：当用户选择的文件夹已被其他项目绑定时，弹出确认提示「该项目已在用 XX API，是否切换？」
- **保留全局默认实例**：设置页面中可配置一个「全局默认目录」作为 fallback（默认仍为 `~/.claude`），用于未指定项目的场景
- **托盘菜单兼容**：托盘菜单的「启动平台」功能改为：点击后弹出文件夹选择框再注入（保持与主界面一致的新交互）

## Tech Stack Selection

- **前端框架**：Vue 3 + TypeScript（已有，沿用）
- **UI 组件库**：Naive UI（已有，沿用）
- **状态管理**：Pinia（已有，沿用）
- **桌面框架**：Tauri 2.x + Rust（已有，沿用）
- **文件对话框**：`@tauri-apps/plugin-dialog`（已安装，用于文件夹选择）
- **加密**：AES-256-GCM（已有 crypto.rs，沿用）

## Tech Architecture

### 系统架构变更概览

```
[当前架构]                          [新架构]
ProviderGrid click                   ProviderGrid click
    |                                    |
    v                                    v
doSwitch(p) → switchProvider     doSwitch(p) → open folder dialog
    |                                    |
    v                                    v
switch_provider(config_dir)       user selects project dir
(固定 ~/.claude)                        |
                                   v
                              check if dir already bound?
                               /                \
                           [yes]              [no]
                              |                  |
                    confirm dialog?      inject_to_project(dir)
                              |                  |
                         inject              record project
```

### 数据模型变更

**新增 ActiveProject 数据结构**（替代原 Instance 的核心角色）：

```typescript
interface ActiveProject {
  id: string               // 唯一 ID，如 "proj_1745xxx"
  name: string             // 项目名称（取自文件夹名或用户编辑）
  project_path: string     // 项目根目录绝对路径，如 "D:/GitWork/my-project"
  provider_id: string      // 当前绑定到的 Provider ID
  created_at: string       // 绑定时间 ISO
  updated_at: string       // 最后更新时间 ISO
}
```

**AppConfig 变更**：

```typescript
interface AppConfig {
  language: string
  default_config_dir?: string          // 全局默认目录（可选，默认 ~/.claude）
  active_projects: ActiveProject[]     // 已激活的项目列表（替换原 instances 数组的主导地位）
}
```

> 原有的 `instances` 字段降级为仅用于 Settings 页面的「全局默认目录」配置，不再作为主流程的核心概念。为向后兼容，保留 `instances` 字段但标记为 legacy，新数据全部走 `active_projects`。

### 核心数据流（新交互）

1. 用户点击 Provider 图标 → `ProviderGrid emit('switch', p)`
2. `AppContent.doSwitch(p)` → 调用 `open({ directory: true, multiple: false })` 弹出文件夹选择框
3. 用户选中目录 → 检查该路径是否已在 `active_projects` 中存在

- **不存在**：直接调用新的 Rust command `inject_to_project(project_path, provider_id)`
- **已存在**：弹出 Naive UI Dialog 确认「该项目已在使用 {old_name} API，是否切换？」→ 用户确认后才执行注入

4. Rust 后端 `inject_to_project`:

- 解密 API Key
- 调用已有的 `inject::inject()` 函数（传入 `{projectPath}/.claude` 作为 config_dir）
- 更新 `active_projects` 记录（新增 or 更新）

5. 前端刷新项目列表

### Module Division

| 模块 | 文件 | 变更类型 | 职责 |
| --- | --- | --- | --- |
| Store | `src/stores/app.ts` | **MODIFY** | 新增 ActiveProject 类型、activeProjects state、project CRUD 方法、loadActiveProjects 等 |
| 主逻辑容器 | `src/components/AppContent.vue` | **MODIFY** | 重写 doSwitch() 为「选文件夹+确认+注入」流程；集成 ProjectList 面板 |
| Provider网格 | `src/components/ProviderGrid.vue` | 微调 | 无需大改，保持现有点击行为 |
| 项目列表组件 | `src/components/ProjectList.vue` | **NEW** | 展示已激活项目卡片、移除按钮、Provider 显示 |
| 设置页 | `src/components/Settings.vue` | **MODIFY** | 将 Instance 管理区域精简为「全局默认目录」配置 |
| Rust 配置模块 | `src-tauri/src/config.rs` | **MODIFY** | 新增 ActiveProject 结构体、序列化、读写方法 |
| Rust 注入模块 | `src-tauri/src/inject.rs` | 微调 | 无需改动，inject() 本身就接收任意 config_dir 参数 |
| Rust 命令层 | `src-tauri/src/lib.rs` | **MODIFY** | 新增 `inject_to_project`、`get_active_projects`、`remove_active_project` 命令；修改托盘菜单逻辑 |
| 国际化 | `src/i18n/zh.ts`, `src/i18n/en.ts` | **MODIFY** | 新增多项目相关翻译 key |


## Implementation Details

### 关键执行细节

1. **文件夹选择器**：使用已有的 `@tauri-apps/plugin-dialog` 的 `open()` 方法，参数 `{ directory: true, title: '选择项目文件夹' }`，返回选中的目录路径字符串

2. **项目路径规范化**：Windows 下路径可能包含 `\` 和 `/`，统一在 Rust 侧做 normalize（`std::path::PathBuf` 的 canonicalize 或简单 replace）

3. **项目名称生成规则**：优先使用所选目录的最后一级文件夹名（如 `D:\Work\my-project` → `my-project`），后续可在 ProjectList 中允许用户重命名

4. **inject.rs 的复用**：现有的 `inject(config_dir, provider, api_key_plain)` 接收一个 config_dir 参数，原本传的是 `~/.claude`，现在只需改为传 `{selected_dir}/.claude` 即可，**无需修改 inject.rs 内部逻辑**

5. **数据持久化**：`active_projects` 存入 `~/.mmycs/config.json` 的 `active_projects` 字段（和现有的 providers、instances 同级）

6. **向后兼容处理**：

- 首次加载时若 `config.json` 中无 `active_projects` 字段，初始化为空数组
- 若有旧版 `instances` 数据且其中第一个 instance 有 `active_provider_id`，可迁移为一条 active_project（可选增强）

7. **托盘菜单适配**：托盘菜单中点击 Provider 名称的行为从「直接切换到默认实例」改为「弹出文件夹选择框」。但考虑到托盘场景可能希望更快捷，方案是：托盘点击时也走同样的「选文件夹」流程，或者提供一个「最近使用」快捷入口（本次先实现前者）

8. **窗口高度调整**：主界面需要展示 ProjectList 区域，建议将初始高度从 620 调整为 720px，或在 content 区域加 scroll

9. **blast radius 控制**：

- `inject.rs`: **零改动**
- `crypto.rs`: **零改动**
- `ProviderForm.vue`, `QuickSetup.vue`: **零改动**
- 仅改 AppContent.vue、app store、config.rs、lib.rs、Settings.vue、ProjectList.vue(新)、i18n

### 性能考虑

- active_projects 列表通常 < 20 条，全量存储在内存即可，无性能瓶颈
- 每次 inject 操作涉及一次文件 IO（settings.json 读写）+ 一次 JSON 序列化（config.json 更新），均在毫秒级完成
- 无需引入缓存或虚拟滚动

## Directory Structure Summary

本实施方案通过以下文件变更将 MMYCodeSwitch-API 从「单实例切换器」升级为「多项目 API 管理器」：

```
d:/GitWork/MMYCodeSwitch-API/
├── src/
│   ├── stores/
│   │   └── app.ts                          # [MODIFY] 新增 ActiveProject 接口、activeProjects ref、
│   │                                         #   addProject/updateProject/removeProject/loadProjects 方法
│   ├── components/
│   │   ├── AppContent.vue                   # [MODIFY] 重写 doSwitch(): 弹文件夹选择框 → 检测重复 → 确认 → 注入；
│   │                                         #   集成 ProjectList 组件；调整布局以容纳项目列表
│   │   ├── ProjectList.vue                  # [NEW] 已激活项目列表组件：展示项目卡片(名称/路径/Provider/时间)，
│   │                                         #   支持移除项目、折叠/展开；响应式设计适配深色模式
│   │   ├── ProviderGrid.vue                 # [微调] 保持不变，仅确保 switch 事件正确传递
│   │   ├── Settings.vue                     # [MODIFY] 将 Instance 管理区简化为"全局默认目录"单项配置，
│   │                                         #   移除多 Instance 增删功能（被 active_projects 替代）
│   │   ├── ProviderForm.vue                 # [无变更]
│   │   └── QuickSetup.vue                   # [无变更]
│   └── i18n/
│       ├── zh.ts                            # [MODIFY] 新增：项目管理相关中文翻译 key 约15个
│       ├── en.ts                            # [MODIFY] 新增：项目管理相关英文翻译 key 约15个
│       └── index.ts                         # [无变更]
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                           # [MODIFY] 新增 Tauri commands: inject_to_project,
│   │                                         #   get_active_projects, remove_active_project;
│   │                                         #   修改 setup_tray 托盘菜单点击逻辑；
│   │                                         #   注册新 commands 到 invoke_handler
│   │   ├── config.rs                         # [MODIFY] 新增 ActiveProject 结构体(serde序列化)，
│   │                                         #   AppConfig 增加 active_projects 字段，
│   │                                         #   新增 load/save/delete active_project 辅助函数
│   │   ├── inject.rs                         # [无变更] inject() 接口不变，只需调用方传入不同的 config_dir
│   │   ├── crypto.rs                         # [无变更]
│   │   └── main.rs                          # [无变更]
│   └── capabilities/
│       └── default.json                     # [可能微调] 确认 dialog 权限已包含（已有 plugin-dialog）
├── index.html                               # [无变更]
├── package.json                             # [无变更]
└── vite.config.ts                           # [无变更]
```

## 设计风格定位

采用 **Modern Glassmorphism + Clean Dashboard** 风格，融合 Naive UI 的原生设计语言，打造专业级的开发者工具界面。界面整体呈现「控制台 / 仪表盘」的感觉，信息层次分明，操作直觉化。

## 页面规划

### Page 1: 主界面（Main Page）— 核心改造页面

这是本次改造的重点页面，需要在原有 Provider Grid 基础上增加项目管理能力。

#### Block 1: 自定义标题栏（已有，保持不变）

- 应用名称 "MMYCodeSwitch-API" + 最小化/最大化/关闭按钮
- 保持拖拽区域

#### Block 2: Provider Grid 区域（已有，基本不变）

- 展示所有已配置的 API 供应商图标卡片网格
- 点击触发「选择项目文件夹 → 注入」的新流程
- 卡片 hover/active 效果保持现有绿色主题

#### Block 3: 已激活项目列表区（NEW - 核心新增区域）

- 位于 Provider Grid 下方、底部工具栏上方
- 标题行：「已打开的项目 (N)」，右侧可收起/展开
- 项目卡片横向排列（每卡显示）：
- 项目文件夹图标 + 项目名称
- 使用的 Provider 图标 + 名称（小字）
- 绑定时间（相对时间，如「2小时前」）
- 移除按钮（x 图标，hover 出现）
- 空状态提示：「尚未绑定项目，点击上方 Provider 图标开始」
- 卡片风格与 Provider Card 统一（圆角14px，浅边框，hover 浮起）
- 深色模式完整适配

#### Block 4: 底部工具栏（已有，微调）

- 快速配置按钮、深色切换、设置按钮保持不变
- 可能增加一个「项目管理」快捷按钮（可选，如果空间够的话）

#### Block 5: 状态栏（已有，保持不变）

- 右键提示文字

### Page 2: 设置页面（Settings Page）— 精简改造

#### Block 1: 页面标题栏（已有，不变）

#### Block 2: 语言设置（已有，不变）

#### Block 3: 全局默认目录（改造自原 Instance 管理）

- 从原来的「多实例列表」精简为一个「全局默认目录」输入框
- 说明文字：「未选择具体项目时的 fallback 目录，留空则使用 ~/.claude」
- 旁边一个「浏览」按钮可选择目录

#### Block 4: 导入导出备份（已有，不变）

#### Block 5: 底部操作栏（已有，不变）