# MMYCodeSwitch-API 开发文档

> 版本：v0.1.0-draft  
> 日期：2026-04-20  
> 作者：yuyeming0115

---

## 一、项目概述

MMYCodeSwitch-API 是一款跨平台桌面应用，用于管理和快速切换 Claude Code CLI 所使用的 API 供应商配置（第三方中转站、官方 API、账号登录模式）。

**核心目标：**
- 一键切换不同 API 供应商，无需手动编辑任何配置文件
- 完全不破坏 `.claude/` 目录的默认结构和其他 CLI 工具的配置
- 独立管理空间，配置可导入导出备份
- 支持 Windows / macOS / Linux
- 支持多个 Claude Code 实例（多 `CLAUDE_CONFIG_DIR` 路径）
- 中英双语 UI，随时切换

---

## 二、痛点与需求分析

| 痛点 | 解决方案 |
|------|----------|
| 各中转站 Base URL、API Key 格式不统一 | 供应商模板 + 粘贴解析自动识别 |
| 每次手动编辑 settings.json 繁琐 | GUI 一键切换，后台自动写入 |
| 换电脑/备份困难 | 配置导入导出（加密 JSON） |
| API Key 安全性 | 输入时明文，保存后星号遮盖；编辑时留空不覆盖 |

---

## 三、独立管理空间设计

参考 JCode 的 `C:\Users\EDY\.jcode\` 目录结构，MMYCodeSwitch-API 使用独立目录，**不污染** `.claude/` 任何文件。

```
~/.mmycs/                          # 独立管理空间（跨平台）
├── config.json                    # App 全局配置（当前激活供应商、语言、实例列表等）
├── providers/                     # 供应商配置目录
│   ├── provider_001.json
│   ├── provider_002.json
│   └── ...
├── instances/                     # 多 Claude Code 实例配置
│   ├── default.json               # 默认实例（~/.claude/）
│   ├── instance_work.json         # 自定义实例
│   └── ...
├── backups/                       # 切换前自动备份 settings.json
│   └── settings_20260420_024800.json
└── logs/                          # 切换操作日志
    └── switch.log
```

**关键原则：** App 只向目标实例的 `settings.json` 的 `env` 字段写入/清除指定环境变量，其余字段原样保留。

---

## 四、配置注入机制

切换供应商时的操作流程：

```
选择目标供应商 + 目标实例
        ↓
备份当前 settings.json → ~/.mmycs/backups/
        ↓
读取当前 settings.json（完整内容）
        ↓
仅替换 env 中的以下字段：
  ANTHROPIC_AUTH_TOKEN
  ANTHROPIC_BASE_URL
  ANTHROPIC_MODEL（可选）
  ANTHROPIC_DEFAULT_HAIKU_MODEL（可选）
  ANTHROPIC_DEFAULT_SONNET_MODEL（可选）
  ANTHROPIC_DEFAULT_OPUS_MODEL（可选）
  ANTHROPIC_REASONING_MODEL（可选）
        ↓
写回 settings.json（其余字段不变）
        ↓
记录切换日志
```

**登录模式：** 切换到官方账号登录模式时，删除上述所有 env 字段，Claude Code 自动回退账号登录。

**禁止触碰的字段：**
```
permissions / enabledPlugins / extraKnownMarketplaces
statusLine / hooks / model（顶层）/ 其他所有非 env 字段
```

---

## 五、供应商数据结构

```json
{
  "id": "provider_001",
  "name": "How88 中转站",
  "icon": "how88_logo.png",
  "iconFallback": "H8",
  "type": "api",
  "baseUrl": "https://how88.top",
  "apiKeyEncrypted": "<Base64(AES-256-GCM 加密后的 key)>",
  "models": {
    "default": "claude-opus-4-6",
    "haiku": "claude-opus-4-6",
    "sonnet": "claude-opus-4-6",
    "opus": "claude-opus-4-6",
    "reasoning": "claude-opus-4-6"
  },
  "notes": "",
  "createdAt": "2026-04-20T02:40:00Z",
  "updatedAt": "2026-04-20T02:40:00Z"
}
```

登录模式：

```json
{
  "id": "provider_login",
  "name": "Claude 官方账号",
  "icon": "claude_logo.png",
  "iconFallback": "CC",
  "type": "login"
}
```

---

## 六、API Key 安全策略

加密目标：**防止截图/屏幕录制时 Key 泄露，防止配置文件被直接读取**，不需要硬件级绑定。

| 阶段 | 处理方式 |
|------|----------|
| 输入时 | 明文显示（方便核对粘贴） |
| 保存后界面显示 | `sk-****...****`（仅显示前4后4位） |
| 编辑时 | 输入框默认空，placeholder：`留空则不更新 / Leave blank to keep current` |
| 本地存储 | AES-256-GCM 加密，密钥由 App 安装时随机生成并存入系统密钥链（Keychain/Credential Manager） |
| 导出备份 | 用户自设备份密码二次加密 Key |
| 导入时 | 验证备份密码解密，再用本机密钥重新加密存储 |

---

## 七、供应商图标规范

优先级：
1. **供应商官方图标**（内置常见供应商：Anthropic、OpenRouter、阿里云百炼、MiniMax、Kimi、智谱 Zai、腾讯云、how88 等）
2. **用户上传图片**（支持 PNG/SVG，自动裁剪为圆角方形）
3. **文字缩写替代**（取名称前2个字符，深色背景，如 `H8`、`OR`、`ALI`）

---

## 八、粘贴解析自动识别

用户粘贴供应商提供的配置代码，App 尝试提取关键字段：

| 输入格式 | 识别规则 |
|----------|----------|
| JSON（含 `ANTHROPIC_BASE_URL`） | 直接解析对应字段 |
| `export ANTHROPIC_BASE_URL=...` | shell 变量格式解析 |
| 纯 URL 行（`https://` 开头） | 提取为 baseUrl 候选 |
| `sk-` 开头字符串 | 提取为 apiKey 候选 |
| 无前缀长字符串（>20位） | 作为 apiKey 候选，提示用户确认 |

解析后展示预览，**用户确认后才保存**，不自动静默写入。

---

## 九、多实例支持

用户可在 App 中管理多个 Claude Code 实例（对应不同的 `CLAUDE_CONFIG_DIR`）：

```json
// ~/.mmycs/instances/instance_work.json
{
  "id": "instance_work",
  "name": "工作环境",
  "configDir": "D:/Work/.claude",
  "activeProviderId": "provider_002"
}
```

切换时选择目标实例，App 向对应目录的 `settings.json` 注入配置。

---

## 十、UI 设计规范

### 10.1 主界面（参考 JCode 网格布局）

```
┌─────────────────────────────────────────────────┐
│  MMYCodeSwitch-API    [实例选择▼]  [🌐 EN] [⚙]  │
├─────────────────────────────────────────────────┤
│  当前激活：How88 中转站  ✅                      │
├─────────────────────────────────────────────────┤
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐           │
│  │[图标]│ │[图标]│ │[图标]│ │[图标]│           │
│  │Claude│ │How88 │ │ 2API │ │ ALI  │           │
│  └──────┘ └──────┘ └──────┘ └──────┘           │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐           │
│  │[图标]│ │[图标]│ │  +   │           │
│  │ Kimi │ │ OR   │ │ 添加 │           │
│  └──────┘ └──────┘ └──────┘           │
├─────────────────────────────────────────────────┤
│  [导出配置]  [导入配置]                          │
└─────────────────────────────────────────────────┘
```

- 点击供应商卡片 = 激活（一键切换）
- 长按/右键卡片 = 编辑 / 删除
- 当前激活的卡片高亮显示 ✅

### 10.2 添加/编辑供应商表单（参考 JCode 添加平台面板）

字段：
- 平台名称（必填）
- API Key（输入明文，保存后遮盖；编辑时留空不更新）
- Base URL（必填，API 模式）
- 默认模型（可选）
- 配置目录（对应实例，可选）
- 额外启动参数（可选）
- 备注
- "从配置代码解析" 按钮（粘贴解析入口）

### 10.3 设置面板

- 语言切换：中文 / English
- 终端偏好（优先使用 Windows Terminal / iTerm2 等）
- 数据管理：导出配置 / 导入配置
- 实例管理：添加/删除 Claude Code 实例
- 关于

---

## 十一、技术选型

| 层级 | 推荐方案 | 备选 |
|------|----------|------|
| 跨平台框架 | **Tauri 2.x**（Rust + WebView） | Electron |
| 前端 UI | **Vue 3 + Vite** | React |
| UI 组件库 | **Naive UI** | Element Plus |
| 加密 | **AES-256-GCM**（Web Crypto API） + 系统密钥链 | — |
| 配置文件读写 | Tauri fs plugin | — |
| 系统密钥链 | `keytar`（跨平台 Keychain/Credential Manager） | — |
| 构建/打包 | Tauri bundler（.exe / .dmg / .deb / .AppImage） | — |

选择 Tauri 的理由：安装包体积小（~5MB vs Electron ~100MB），原生系统 API 访问更安全。

---

## 十二、代码仓库目录结构

```
MMYCodeSwitch-API/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs        # 配置读写
│   │   ├── crypto.rs        # AES-256-GCM 加密解密
│   │   ├── inject.rs        # settings.json 注入逻辑
│   │   └── instance.rs      # 多实例管理
│   └── tauri.conf.json
├── src/
│   ├── components/
│   │   ├── ProviderGrid.vue  # 主界面网格
│   │   ├── ProviderForm.vue  # 添加/编辑表单
│   │   ├── PasteParser.vue   # 粘贴解析
│   │   └── Settings.vue      # 设置面板
│   ├── stores/
│   │   ├── providers.ts
│   │   └── instances.ts
│   ├── i18n/
│   │   ├── zh.ts
│   │   └── en.ts
│   └── App.vue
├── 参考资料/
└── MMYCodeSwitch-API开发文档.md
```

---

## 十三、开发阶段规划

### Phase 1 — MVP
- [ ] Tauri + Vue 3 项目脚手架
- [ ] `~/.mmycs/` 独立空间初始化
- [ ] 供应商 CRUD + 网格 UI（内置官方图标 + 文字缩写兜底）
- [ ] API Key AES-256-GCM 加密存储
- [ ] 一键切换（注入 env 到 settings.json）
- [ ] 登录模式支持（清除 env）
- [ ] 切换前自动备份 settings.json
- [ ] 多实例管理（多 `CLAUDE_CONFIG_DIR` 支持）

### Phase 2 — 完善体验
- [ ] 粘贴解析自动识别
- [ ] 导入导出备份（备份密码加密）
- [ ] 中英双语 i18n
- [ ] 用户上传自定义供应商图片

### Phase 3 — 增强功能
- [x] 供应商连通性测试（发送最小请求验证 Key）
- [x] 系统托盘快速切换
- [x] 自动检测已安装的 Claude Code 实例路径

---

## 十四、CLAUDE.md 模板管理（v3.5 新增）

### 14.1 功能概述

不同项目有不同的开发规范，CLAUDE.md 模板管理功能允许用户：
- 创建/编辑/删除 CLAUDE.md 模板
- 将模板绑定到特定项目
- 一键将模板内容写入项目的 `.claude/CLAUDE.md`

### 14.2 存储结构

```
~/.mmycs/
├── templates/                    # CLAUDE.md 模板目录
│   ├── vue-standard.md           # Vue 项目标准模板
│   ├── python-clean.md           # Python 无注释风格
│   ├── react-ts.md               # React TypeScript 规范
│   └── custom-xxx.md             # 用户自定义
├── template_bindings.json        # 项目-模板绑定关系
└── ...
```

### 14.3 模板数据结构

```json
// ~/.mmycs/templates/vue-standard.md
# Vue 3 项目开发规范
- 使用 Composition API
- 组件命名：PascalCase
- ...

// ~/.mmycs/template_bindings.json
{
  "bindings": [
    {
      "project_path": "D:/Projects/my-vue-app",
      "template_name": "vue-standard",
      "updated_at": "2026-04-22T10:00:00Z"
    }
  ]
}
```

### 14.4 UI 设计

在设置页面新增「开发规则模板」区域：
- 模板列表（可编辑/删除）
- 新建模板按钮
- 项目绑定管理

---

## 十五、Skill 模板库（v3.5 新增）

### 15.1 功能概述

Skill 是 Claude Code 的扩展机制，存储在 `~/.claude/skills/` 目录。Skill 模板库功能：
- 管理常用 skill 模板
- 查看/编辑/删除 skill
- 备份 skill 配置

### 15.2 存储结构

```
~/.mmycs/
├── skills/                       # Skill 模板目录
│   ├── simplify.md               # 代码简化 skill
│   ├── review.md                 # PR 审查 skill
│   └── custom-xxx.md             # 用户自定义
└── ...
```

### 15.3 Skill 格式

```markdown
# simplify

Review changed code for reuse, quality, and efficiency, then fix any issues found.

## When to use
- After making code changes
- Before committing changes

## Instructions
1. Check for duplicate code
2. Verify naming conventions
3. Ensure no unused imports
```

---

## 十六、完整备份方案（v3.5 新增）

### 16.1 功能概述

完善导出功能，支持跨电脑完整迁移，包含：
- API 供应商配置 ✓ 已有
- CLAUDE.md 模板
- Skill 模板
- 应用偏好设置

### 16.2 完整备份文件结构

```
完整备份文件 (.mmycs 二进制格式)
├── [Magic: "MMYCS"]              # 文件标识
├── [Version: 0x03]               # 版本号（v3 支持完整备份）
├── [Machine key hash]            # 机器识别
├── [Password flag]               # 是否密码保护
├── providers/                    # API 供应商配置（加密）
├── templates/                    # CLAUDE.md 模板
├── skills/                       # Skill 模板
├── app_config/                   # 应用偏好设置
│   ├── language
│   ├── backupExportPath
│   └── defaultConfigDir
└── template_bindings/            # 项目-模板绑定关系
```

### 16.3 导入逻辑

1. 解析备份文件头，识别版本
2. 检查机器匹配：
   - 同机 + 无密码 → 自动导入
   - 跨机器 + 有密码 → 输入密码导入
   - 跨机器 + 无密码 → 无法导入
3. 恢复所有配置到对应目录

---

## 十七、目录结构更新

```
~/.mmycs/                         # 独立管理空间（跨平台）
├── config.json                   # App 全局配置
├── .key                          # 加密密钥
├── providers/                    # 供应商配置目录
│   └── provider_xxx.json
├── templates/                    # CLAUDE.md 模板目录（新增）
│   └── xxx.md
├── skills/                       # Skill 模板目录（新增）
│   └── xxx.md
├── template_bindings.json        # 项目-模板绑定（新增）
├── backups/                      # 切换前自动备份
│   └── settings_xxx.json
│   └── mmycs_backup_xxx.mmycs    # 完整备份文件
├── projects/                     # 项目专属配置目录
│   └── {project_hash}/
│       ├── settings.json
│       ├── binding.json
│       └── sessions/
├── icons/                        # 自定义图标
├── logs/                         # 操作日志
└── window_state.json             # 窗口状态
```

---

## 十八、开发阶段规划更新

### Phase 4 — 完整备份（v3.5）
- [ ] 完善导出功能：包含模板 + skill + 应用配置
- [ ] 版本升级至 0x03
- [ ] 导入时自动恢复所有数据

### Phase 5 — CLAUDE.md 模板管理（v3.5）
- [ ] 模板 CRUD 接口
- [ ] 模板列表 UI
- [ ] 项目绑定功能
- [ ] 一键注入 CLAUDE.md 到项目

### Phase 6 — Skill 模板库（v3.5）
- [ ] Skill CRUD 接口
- [ ] Skill 列表 UI
- [ ] Skill 备份/恢复

---

## 十九、变更记录

### 2026-05-12 开发记录

#### 1. 供应商目录结构优化（v1→v2 自动迁移）
- **提交：** ec9353d
- **涉及文件：** `src-tauri/src/config.rs`, `src-tauri/src/inject.rs`, `src-tauri/src/lib.rs`
- **方案：** 供应商目录结构从 v1 升级到 v2，实现自动迁移逻辑，兼容旧版配置格式
- **修复：** 提交 e4b6449 修复了迁移逻辑中的两个问题（`config.rs`, `lib.rs`）

#### 2. i18n 修复
- **提交：** 414d3e6
- **涉及文件：** `src/i18n/zh.ts`
- **内容：** 修复中文翻译文件中重复的 i18n key

#### 3. UI 主题全面改版 — Claude Code 橙色主题
- **提交：** e3faf20
- **涉及文件：** `AppContent.vue`, `ProjectList.vue`, `ProviderGrid.vue`, `i18n/en.ts`, `i18n/zh.ts`
- **内容：**
  - UI 全面改用 Claude Code 标志性橙色主题
  - toolbar 操作栏从顶部移至底部
  - 添加供应商按钮移至 toolbar 首位，悬浮时高亮提示（67b6054）

#### 4. 供应商卡片交互优化
- **提交：** 7c413bf, cdb0b36
- **涉及文件：** `ProviderGrid.vue`, `ProviderForm.vue`
- **内容：**
  - 卡片 hover 时放大浮起效果
  - 按钮文字切换为"启动 Claude Code"
  - 供应商卡片图标显示前 3 个字母（原为 2 个）

#### 5. 页面布局重构
- **提交：** a7a9cf3, e49a88c, d786a7c, 532247a
- **涉及文件：** `App.vue`, `AppContent.vue` 及所有子页面组件
- **内容：**
  - 页面标题整合到标题栏，去掉原有的底部栏
  - 新增自动保存功能
  - 统一子页面底部栏高度，优化模板卡片图标样式
  - 标题栏主页面和子页面采用差异化布局
  - 标题栏按钮风格统一为底部 toolbar 圆角 SVG 风格

#### 6. 主题色取色器
- **提交：** 765f409, 728443f
- **涉及文件：** `App.vue`, `AppContent.vue`, `ProjectList.vue`, `ProviderForm.vue`, `ProviderGrid.vue`
- **内容：**
  - 新增主题色取色器按钮，CSS 变量统一管理主题色
  - 取色器改用 Naive UI 原生组件，替代自定义实现
  - 优化深色模式下的 modal 样式

#### 7. 工具栏拖拽功能
- **涉及文件：** `src/components/AppContent.vue`
- **内容：**
  - 底部工具栏空白区域支持左键按住拖动窗体
  - 将拖拽排除列表中的 `.toolbar` 替换为 `.toolbar-btn`，按钮本身仍不可拖拽

### 2026-05-11 开发记录

#### 1. 安全改进计划
- **文件：** `安全改进计划-APIKey防泄露与注入防污染.md`
- **内容：** API Key 防泄露与注入防污染方案设计

#### 2. jCode 机制与目录结构优化
- **文件：** `.jcode机制提炼与采纳方案.md`, `.jcode目录结构优化方案.md`
- **内容：** 研究 jCode 的目录结构和管理机制，为 MMYCodeSwitch-API 的目录设计提供参考

#### 3. 项目文档更新
- **内容：** CHANGELOG.md 更新，CLAUDE.md 项目管理
