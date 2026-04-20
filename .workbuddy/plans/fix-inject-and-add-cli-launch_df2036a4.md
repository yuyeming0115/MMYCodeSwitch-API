---
name: fix-inject-and-add-cli-launch
overview: 排查多项目注入无反应的 Bug，修复后新增注入完成自动弹出 PowerShell 启动 Claude Code CLI 的功能
design:
  architecture:
    framework: vue
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
  - id: fix-inject-bug
    content: 修复注入无反应 Bug：在 AppContent.vue 的 doSwitch/doInject 全链路添加 console.log/error 日志和 loading 状态，排查并解决 toast 不显示的问题
    status: completed
  - id: add-launch-cli-rust
    content: 在 lib.rs 新增 launch_terminal Tauri command：注册 tauri_plugin_shell，实现用 PowerShell/WTerminal 在指定工作目录启动 claude 命令
    status: completed
  - id: add-launch-cli-permission
    content: 在 capabilities/default.json 添加 shell:allow-execute 权限配置
    status: completed
    dependencies:
      - add-launch-cli-rust
  - id: integrate-launch-cli
    content: 修改 AppContent.vue 的 doInject 函数：注入成功后自动调用 launch_terminal 并显示启动提示
    status: completed
    dependencies:
      - fix-inject-bug
      - add-launch-cli-rust
  - id: update-i18n-cli
    content: 补充 i18n 国际化文案：zh.ts/en.ts 新增 CLI 启动相关翻译 key（launching_cli、launch_failed、injecting 等）
    status: completed
---

## Product Overview

MMYCodeSwitch-API 多项目模式改造的第二阶段：修复「点击供应商选择文件夹后无任何反应」的 Bug，并新增「注入完成后自动启动 Claude Code CLI 终端」功能。保持现有的「选择任意项目文件夹注入」架构不变。

## Core Features

- **修复注入静默失败 Bug**：当前用户选择项目文件夹后完全没有视觉反馈（无 toast、无列表更新），需要排查并修复根因，同时增加 loading 状态和详细日志
- **自动启动 Claude Code CLI**：API 注入到目标项目的 `.claude/settings.json` 后，自动弹出终端窗口（PowerShell），cd 到目标项目目录并执行 `claude` 命令
- **注入过程可视化**：在注入过程中显示 loading 指示器，完成后显示明确的成功/失败提示
- **项目列表实时刷新**：注入成功后立即在「已打开的项目」列表中显示新绑定的项目卡片

## 用户操作期望（对比 JCode）

用户希望达到的效果：

1. 点击供应商图标 → 弹出文件夹选择框
2. 选择项目文件夹 → 显示 loading → 注入配置到 `{project}/.claude/settings.json`
3. **注入成功后自动弹出 PowerShell 窗口**，已在目标项目目录下运行 `claude` 命令
4. 主界面下方「已打开的项目」列表中显示该项目卡片

## Tech Stack

- **前端框架**：Vue 3 + TypeScript + Pinia（已有）
- **UI 组件库**：Naive UI（已有，使用其 Message / Dialog / Loading 组件）
- **桌面框架**：Tauri 2.x + Rust（已有）
- **Shell 能力**：`tauri-plugin-shell = "2"`（Cargo.toml 已有依赖，需注册启用）
- **文件对话框**：`@tauri-apps/plugin-dialog`（已正常工作）

## Implementation Approach

### Bug 根因诊断与修复策略

通过逐层分析代码链路，定位以下潜在问题点：

**问题 A：前端调用链路缺乏防御性日志**
`AppContent.vue` 的 `doSwitch()` 和 `doInject()` 函数中，如果 `store.injectToProject()` 抛出异常，catch 块虽然会调用 `msg.error()`，但如果 Naive UI Message 组件渲染异常或被遮挡，用户就看不到任何反馈。此外，没有 `console.error` 输出导致无法在 DevTools 中排查。

**修复方案**：在每个异步调用点添加 `console.log/console.error` 日志，并在注入前后分别添加 loading 状态。

**问题 B：Rust 侧 `inject_to_project` 可能静默失败**
`inject::inject()` 函数内部调用 `std::fs::create_dir_all()` 和 `std::fs::write()`，如果在 Windows 下遇到路径长度限制（MAX_PATH 260字符）、权限问题、或路径包含特殊字符，这些操作可能返回 Err 但错误信息不够明确。

**修复方案**：在 Rust command 中对关键 IO 操作增加更具体的错误上下文信息（如拼接路径到错误消息）。

### 自动启动 CLI 功能实现方案

**策略选择**：使用 Tauri Shell 插件（`tauri-plugin-shell`）的 `Command::new()` API 创建 Sidecar 或直接命令。

具体实现路径：

1. **Rust 层**：新建 `launch_terminal` Tauri command，接收目标工作目录路径作为参数

- Windows 平台：使用 `std::process::Command` 启动 `powershell.exe` 或 `wt.exe`（Windows Terminal），参数为 `-NoExit -Command "Set-Location '{workdir}'; claude"`
- 使用 `current_dir(workdir)` 设置工作目录更可靠
- 进程不等待（非阻塞），让终端窗口独立运行

2. **前端层**：在 `doInject()` 成功后调用 `invoke('launch_terminal', { workDir: selectedPath })`

3. **权限配置**：在 `capabilities/default.json` 添加 `shell:allow-execute` 权限

**为什么不使用 Shell 插件的 Command API 而用原生 std::process**：

- Shell 插件的 Command API 更适合管理应用内子进程生命周期
- 我们需要的是「启动一个独立的终端窗口并脱离父进程」，`std::process::Command` 更直接可控
- 避免引入额外的 sidecar 配置复杂度

### 数据流（修复后的完整流程）

```
用户点击 Provider 图标
       │
       ▼
 doSwitch(p)
       │
       ├── 1. open({directory:true}) → 文件夹选择框
       │         │
       │         ▼ (用户选择后返回路径)
       │
       ├── 2. console.log('[inject] 选择路径:', selectedPath)
       │
       ├── 3. 检查重复绑定 → 必要时弹出确认框
       │
       ├── 4. doInject(path, p)
       │         │
       │         ├── store.injectToProject() → invoke('inject_to_project')
       │         │         │
       │         │         └── [Rust] 写入 settings.json + 更新 active_projects
       │         │
       │         ├── ✅ 成功: msg.success() toast + console.log
       │         │         │
       │         │         └── invoke('launch_terminal', { workDir: path })
       │         │                 │
       │         │                 └── [Rust] 启动 PowerShell 运行 claude
       │         │
       │         └── ❌ 失败: msg.error() + console.error
       │
       └── 5. loadActiveProjects() → ProjectList 刷新显示
```

## Architecture Design

### 模块变更范围

| 文件 | 变更类型 | 变更内容 |
| --- | --- | --- |
| `src-tauri/src/lib.rs` | **MODIFY** | 新增 `launch_terminal` command；注册 `tauri_plugin_shell` |
| `src-tauri/capabilities/default.json` | **MODIFY** | 添加 shell 权限 |
| `src/components/AppContent.vue` | **MODIFY** | 增加 loading 状态、console 日志、注入后调 launch_terminal |
| `src/stores/app.ts` | **MODIFY** | injectToProject 增加日志；可选：新增 launchTerminal 方法 |
| `src/i18n/zh.ts` / `en.ts` | **MODIFY** | 补充 CLI 启动相关翻译 key |
| `src-tauri/src/inject.rs` | **微调** | 错误信息增强（可选） |


## Directory Structure Summary

本次修改聚焦于 Bug 修复 + CLI 启动功能，涉及 6 个文件的精确改动：

```
d:/GitWork/MMYCodeSwitch-API/
├── src/
│   ├── components/
│   │   └── AppContent.vue                   # [MODIFY] 核心：增加 injecting ref 状态、loading UI、
│   │                                         #   console.log/error 全链路日志、doInject 成功后调用
│   │                                         #   launchTerminal；修复 toast 不显示的问题
│   ├── stores/
│   │   └── app.ts                           # [MODIFY] injectToProject 方法增加 console.log；
│   │                                         #   可选新增 launchTerminal() 封装方法
│   └── i18n/
│       ├── zh.ts                            # [MODIFY] 新增: launching_cli、launch_failed 等 key
│       └── en.ts                            # [MODIFY] 同上英文版
├── src-tauri/
│   ├── capabilities/
│   │   └── default.json                     # [MODIFY] 新增 shell:allow-execute 权限
│   └── src/
│       └── lib.rs                           # [MODIFY] .plugin(tauri_plugin_shell::init()) +
│       │                                     #   新增 #[tauri::command] fn launch_terminal()
│       └── inject.rs                        # [微调] 错误信息增加路径上下文（可选）
```

## Implementation Notes (Execution Details)

1. **Grounded 复用现有模式**：新 command `launch_terminal` 遵循现有 `hide_to_tray` 等 command 的相同模式（`#[tauri::command]` + `invoke_handler` 注册）

2. **Loading 状态实现**：使用 Naive UI 的 `useMessage().loading()` 或 Vue ref 控制按钮 disabled 态，避免用户重复点击

3. **Shell 权限最小化**：只授予 `shell:allow-execute`，不授予通配符权限

4. **Windows 终端启动策略**：

- 优先尝试 `wt.exe`（Windows Terminal，现代 Windows 默认自带）
- Fallback 到 `powershell.exe`（兼容性最好）
- 命令参数：`-NoExit -Command "claude"` 配合 `current_dir()` 设定工作目录
- 使用 `creation_flags CREATE_NEW_CONSOLE` 或 `CREATE_NO_WINDOW` 控制窗口行为
- 实际上对于 PowerShell，直接用 `.creation_flags(0x00000010)` (CREATE_NEW_CONSOLE) 即可

5. **Blast radius 控制**：

- `config.rs`、`crypto.rs`：零改动
- `ProjectList.vue`、`ProviderGrid.vue`、`Settings.vue`：零改动
- 所有变更集中在 AppContent.vue、lib.rs、app.ts、i18n、capabilities

6. **性能考虑**：`launch_terminal` 是一次性 `spawn` 操作，不阻塞主线程；`std::process::Command::spawn()` 立即返回

## 设计说明

本次不涉及全新页面设计，而是在现有界面基础上进行交互增强：

### 主界面交互改进

**Block 增强 - Provider Grid 区域交互反馈**：

- 点击 Provider 卡片后，在文件夹选择期间和注入过程中，卡片应显示 loading/disable 状态，防止重复点击
- 注入成功后卡片短暂闪烁绿色边框确认效果

**Block 增强 - 已激活项目列表区**：

- 列表应在注入完成后立即刷新显示新项目（当前可能存在响应式更新延迟）
- 新增的项目卡片可以有一个短暂的「高亮入场」动画（绿色背景渐隐）

**Block 新增 - CLI 启动状态指示**：

- 在底部状态栏或工具栏附近，可以显示一个小的「正在启动终端...」临时提示
- 或者在成功 toast 中包含「正在启动 Claude Code CLI...」的文字