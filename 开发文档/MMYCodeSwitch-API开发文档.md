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
- [ ] 供应商连通性测试（发送最小请求验证 Key）
- [ ] 系统托盘快速切换
- [ ] 自动检测已安装的 Claude Code 实例路径
