<script setup lang="ts">
import { ref, onMounted, computed, watch, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessage, useDialog, NIcon } from 'naive-ui'
import { useAppStore, type Provider } from '../stores/app'
import { i18n } from '../i18n'
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import ProviderGrid from './ProviderGrid.vue'
import ProviderForm from './ProviderForm.vue'
import Settings from './Settings.vue'
import ProjectList from './ProjectList.vue'
import TemplateManager from './TemplateManager.vue'
import SkillManager from './SkillManager.vue'
import PluginManager from './PluginManager.vue'
import TokenStats from './TokenStats.vue'
import {
  StatsChartOutline,
  DocumentTextOutline,
  BuildOutline,
  ExtensionPuzzleOutline,
  SunnyOutline,
  MoonOutline,
  SettingsOutline
} from '@vicons/ionicons5'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

// 当前页面
const currentPage = ref<'main' | 'settings' | 'form' | 'templates' | 'skills' | 'plugins' | 'usage-stats'>('main')
const editingProvider = ref<Provider | undefined>()
const isDark = defineModel<boolean>('isDark', { default: false })

const appWindow = getCurrentWindow()
let resizeTimeout: ReturnType<typeof setTimeout> | null = null
let hourlyBackupTimer: ReturnType<typeof setInterval> | null = null

/// 全局窗口拖拽：按住空白区域即可拖动窗口（macOS 透明窗口 data-tauri-drag-region 不可靠时的兜底）
function startWindowDrag(e: MouseEvent) {
  // 忽略点击在按钮/链接/输入框等交互元素上的情况
  const target = e.target as HTMLElement
  // 注意：ProviderGrid 卡片类名是 .card，ProjectList 卡片类名是 .proj-card
  if (target.closest('button, a, input, select, textarea, [role="button"], .n-button, .n-input, .n-select, .n-checkbox, .n-switch, .card, .proj-card, .n-modal, .n-popover, .n-dropdown, .toolbar, .statusbar, .titlebar, .project-section-toggle')) {
    return
  }
  // 同步调用，避免阻塞 UI
  appWindow.startDragging().catch(() => {
    /* startDragging 在部分平台可能不可用，静默失败 */
  })
}

onMounted(async () => {
  await store.init()
  i18n.global.locale.value = store.config.language as 'zh' | 'en'

  // 恢复深浅色模式和精简模式状态，同时调整窗口大小
  try {
    const saved = await invoke<{ isDark?: boolean; compactMode?: boolean } | null>('get_window_state') as any
    if (saved && typeof saved.isDark === 'boolean') {
      isDark.value = saved.isDark
    }
    if (saved && typeof saved.compactMode === 'boolean') {
      compactMode.value = saved.compactMode
      // 根据精简模式设置窗口大小
      if (saved.compactMode) {
        const height = await calcCompactHeight()
        await appWindow.setSize(new LogicalSize(NORMAL_SIZE.width, height))
      } else {
        await appWindow.setSize(new LogicalSize(NORMAL_SIZE.width, NORMAL_SIZE.height))
      }
    }
  } catch (_) { /* 首次运行无状态文件 */ }

  // 监听窗口大小变化，延迟保存状态（避免频繁写入）
  await appWindow.onResized(async () => {
    if (resizeTimeout) clearTimeout(resizeTimeout)
    resizeTimeout = setTimeout(async () => {
      try {
        const pos = await appWindow.outerPosition()
        const size = await appWindow.outerSize()
        await invoke('save_window_state', { x: pos.x, y: pos.y, width: size.width, height: size.height, isDark: isDark.value, compactMode: compactMode.value })
      } catch (_) {}
    }, 500)
  })

  // 启动每小时自动备份
  startHourlyBackup()
})

onUnmounted(() => {
  if (resizeTimeout) clearTimeout(resizeTimeout)
  if (hourlyBackupTimer) clearInterval(hourlyBackupTimer)
})

// 每小时自动备份（不含插件文件，减小体积）
function startHourlyBackup() {
  hourlyBackupTimer = setInterval(async () => {
    try {
      await invoke('export_full_backup', {
        password: '',
        includeTemplates: true,
        includeSkills: true,
        includePlugins: false,
        customPath: null
      })
      console.log('[AutoBackup] 每小时备份完成')
    } catch (e) {
      console.error('[AutoBackup] 每小时备份失败:', e)
    }
  }, 60 * 60 * 1000) // 1小时
}

// 监听深浅模式变化，自动保存
watch(isDark, async (val) => {
  try {
    const pos = await appWindow.outerPosition()
    const size = await appWindow.outerSize()
    await invoke('save_window_state', { x: pos.x, y: pos.y, width: size.width, height: size.height, isDark: val, compactMode: compactMode.value })
  } catch (_) {}
})

const activeInstance = computed(() => store.activeInstance())
const activeProviderId = computed(() => activeInstance.value?.active_provider_id)
const injecting = ref(false)  // 注入进行中，防止重复点击
const projectListCollapsed = ref(false)  // 项目列表折叠状态
const compactMode = ref(false)  // 精简模式状态
const providerGridRef = ref<InstanceType<typeof ProviderGrid> | null>(null)

// 窗口尺寸配置
const NORMAL_SIZE = { width: 510, height: 620 }

/// 计算精简模式下窗口自适应高度：标题栏 + ProviderGrid 实际内容高度 + 边距
async function calcCompactHeight(): Promise<number> {
  await nextTick()
  // 测量 ProviderGrid 的实际渲染高度
  const gridEl = document.querySelector('.grid.compact') as HTMLElement | null
  if (gridEl) {
    const gridHeight = gridEl.scrollHeight
    // 标题栏 36px + content padding-top 8px + grid 内容 + 底部边距 8px
    return 36 + 8 + gridHeight + 8
  }
  // fallback：如果测量不到，使用固定值
  return 360
}

/// 精简模式下自适应调整窗口大小
async function resizeToCompact() {
  if (!compactMode.value) return
  try {
    const pos = await appWindow.outerPosition()
    const height = await calcCompactHeight()
    await appWindow.setSize(new LogicalSize(NORMAL_SIZE.width, height))
    await invoke('save_window_state', { x: pos.x, y: pos.y, width: NORMAL_SIZE.width, height, isDark: isDark.value, compactMode: true })
  } catch (_) {}
}

// 监听精简模式变化，自动调整窗口大小
watch(compactMode, async (val) => {
  try {
    const pos = await appWindow.outerPosition()
    if (val) {
      // 精简模式：自适应缩放
      const height = await calcCompactHeight()
      await appWindow.setSize(new LogicalSize(NORMAL_SIZE.width, height))
      await invoke('save_window_state', { x: pos.x, y: pos.y, width: NORMAL_SIZE.width, height, isDark: isDark.value, compactMode: val })
    } else {
      // 普通模式：恢复固定尺寸
      await appWindow.setSize(new LogicalSize(NORMAL_SIZE.width, NORMAL_SIZE.height))
      await invoke('save_window_state', { x: pos.x, y: pos.y, width: NORMAL_SIZE.width, height: NORMAL_SIZE.height, isDark: isDark.value, compactMode: val })
    }
  } catch (_) {}
})

// 精简模式下供应商列表变化时，自适应调整窗口大小
watch(() => store.providers.length, async () => {
  if (compactMode.value) {
    await resizeToCompact()
  }
})

async function toggleMax() {
  if (await appWindow.isMaximized()) {
    await appWindow.unmaximize()
  } else {
    await appWindow.maximize()
  }
}

async function doCloseWindow() {
  try {
    await invoke('hide_to_tray')
  } catch (e) {
    // fallback：直接调用前端 hide
    console.error('hide_to_tray failed:', e)
    await appWindow.hide().catch(() => {})
  }
}

function openAdd() {
  editingProvider.value = undefined
  currentPage.value = 'form'
}

function openEdit(p: Provider) {
  editingProvider.value = p
  currentPage.value = 'form'
}

function goBack() {
  currentPage.value = 'main'
  editingProvider.value = undefined
}

function openTemplates() {
  currentPage.value = 'templates'
}

function openSkills() {
  currentPage.value = 'skills'
}

function openPlugins() {
  currentPage.value = 'plugins'
}

/// 多项目模式核心流程：点击 Provider → 弹文件夹选择器 → 检测重复 → 确认 → 注入 → 启动CLI
async function doSwitch(p: Provider) {
  if (injecting.value) return
  console.log('[doSwitch] 点击供应商:', p.name)

  try {
    // 1. 弹出文件夹选择对话框
    const rawPath = await open({
      directory: true,
      multiple: false,
      title: t('select_project_folder'),
    })

    // ★ 关键修复：Tauri v2 dialog open() 可能返回 string | string[]
    const selectedPath = Array.isArray(rawPath) ? rawPath[0] : rawPath

    console.log('[doSwitch] 文件夹选择结果:', { raw: rawPath, resolved: selectedPath })

    if (!selectedPath || selectedPath.length === 0) {
      msg.info(t('select_folder_cancelled'), { duration: 2000 })
      return
    }

    // 2. 检查该路径是否已有绑定项目（防御 project_path 缺失的脏数据）
    const existing = store.activeProjects.find(
      proj => proj.project_path && normalizePath(proj.project_path) === normalizePath(selectedPath)
    )

    // 3a. 如果已存在且是不同 provider，弹出确认框
    if (existing && existing.provider_id !== p.id) {
      return new Promise<void>((resolve) => {
        dialog.warning({
          title: t('confirm_switch_provider', { old: existing.provider_name, new: p.name }),
          content: `${t('project')}: ${existing.name}\n📁 ${existing.project_path}`,
          positiveText: t('confirm'),
          negativeText: t('cancel'),
          onPositiveClick: async () => {
            await doInject(selectedPath, p)
            resolve()
          },
          onNegativeClick: () => resolve(),
        })
      })
    }

    // 3b. 不存在或同一 provider，直接注入
    await doInject(selectedPath, p)
  } catch (e) {
    console.error('[doSwitch] 异常:', e)
    injecting.value = false
    const errMsg = (e instanceof Error ? e.message : String(e))
    msg.error(t('switch_failed') + ': ' + errMsg, { duration: 5000 })
  }
}

/// 执行实际的注入操作 + 自动启动CLI
async function doInject(projectPath: string, p: Provider) {
  injecting.value = true
  console.log('[doInject] 开始注入:', { provider: p.name, projectPath })

  try {
    const loadingMsg = msg.loading(t('injecting'), { duration: 0 })

    const result = await store.injectToProject(projectPath, p.id)
    console.log('[doInject] 注入成功:', result)

    loadingMsg.destroy()
    const projectName = result.project.name
    msg.success(t('inject_success', { provider: p.name, project: projectName }), { duration: 4000 })

    // 🚀 自动启动 Claude Code CLI
    try {
      msg.info(t('launching_cli'), { duration: 3000 })
      await invoke('launch_terminal', { workdir: projectPath })
      console.log('[doInject] CLI 启动成功')
    } catch (cliErr) {
      console.error('[doInject] CLI 启动失败:', cliErr)
      msg.warning(t('launch_failed', { msg: (cliErr as Error).message }), { duration: 4000 })
    }
  } catch (e) {
    console.error('[doInject] 注入失败:', e)
    const errMsg = (e instanceof Error ? e.message : String(e))
    msg.error(t('switch_failed') + ': ' + errMsg, { duration: 5000 })
  } finally {
    injecting.value = false
  }
}

/// 移除项目绑定
async function handleRemoveProject(id: string) {
  await store.removeActiveProject(id)
}

/// 继续开发：在项目目录重新打开 Claude Code CLI
async function handleLaunchProject(projectPath: string) {
  try {
    msg.info(t('launching_cli'), { duration: 3000 })
    await invoke('launch_terminal', { workdir: projectPath })
    console.log('[handleLaunch] CLI 启动成功:', projectPath)
  } catch (e) {
    console.error('[handleLaunch] CLI 启动失败:', e)
    msg.warning(t('launch_failed', { msg: (e instanceof Error ? e.message : String(e)) }), { duration: 4000 })
  }
}

// 工具函数：规范化路径（统一 / 分隔，防御空值）
function normalizePath(path: string | undefined | null): string {
  if (!path) return ''
  return path.replace(/\\/g, '/').replace(/\/+$/, '')
}

function confirmDelete(p: Provider) {
  dialog.warning({
    title: t('confirm_delete'),
    content: p.name,
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: async () => {
      await store.deleteProvider(p.id)
      msg.success(t('delete_success'))
    },
  })
}

// 拖拽重排供应商顺序
async function handleReorder(orderedIds: string[]) {
  await store.reorderProviders(orderedIds)
}

// 拖拽重排项目顺序
async function handleReorderProjects(orderedIds: string[]) {
  await store.reorderActiveProjects(orderedIds)
}
</script>

<template>
  <div class="app" @mousedown="startWindowDrag">
    <!-- 自定义标题栏（data-tauri-drag-region + startDragging 双重保障 macOS 拖拽） -->
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-left">
        <img class="titlebar-icon" src="/icon.png" width="20" height="20" />
        <span class="titlebar-title">MMYCodeSwitch-API</span>
      </div>
      <div class="titlebar-controls">
        <button class="titlebar-btn compact" @click="compactMode = !compactMode" title="精简模式">
          {{ compactMode ? '📚' : '📖' }}
        </button>
        <button class="titlebar-btn" @click="appWindow.minimize()">─</button>
        <button class="titlebar-btn" @click="toggleMax">□</button>
        <button class="titlebar-btn close" @click="doCloseWindow">✕</button>
      </div>
    </div>

    <!-- 主页面 -->
    <div v-if="currentPage === 'main'" class="page-main">
      <!-- 顶部工具栏（简洁矢量图标） -->
      <div v-show="!compactMode" class="toolbar">
        <div class="toolbar-btn" @click="currentPage = 'usage-stats'" title="Token统计">
          <n-icon :size="20"><StatsChartOutline /></n-icon>
        </div>
        <div class="toolbar-btn" @click="currentPage = 'templates'" title="规则模板">
          <n-icon :size="20"><DocumentTextOutline /></n-icon>
        </div>
        <div class="toolbar-btn" @click="currentPage = 'skills'" title="Skills">
          <n-icon :size="20"><BuildOutline /></n-icon>
        </div>
        <div class="toolbar-btn" @click="currentPage = 'plugins'" title="插件">
          <n-icon :size="20"><ExtensionPuzzleOutline /></n-icon>
        </div>
        <div class="toolbar-btn" @click="isDark = !isDark" :title="isDark ? '浅色模式' : '深色模式'">
          <n-icon :size="20"><component :is="isDark ? SunnyOutline : MoonOutline" /></n-icon>
        </div>
        <div class="toolbar-btn" @click="currentPage = 'settings'" title="设置">
          <n-icon :size="20"><SettingsOutline /></n-icon>
        </div>
      </div>

      <!-- 内容区域 -->
      <div class="content">
        <ProviderGrid
          :providers="store.providers"
          :active-provider-id="activeProviderId"
          :compact="compactMode"
          @switch="doSwitch"
          @edit="openEdit"
          @delete="confirmDelete"
          @add="openAdd"
          @reorder="handleReorder"
        />

        <!-- 已打开项目折叠控制 -->
        <div v-show="!compactMode" class="project-section-toggle" @click="projectListCollapsed = !projectListCollapsed">
          <span class="toggle-icon">{{ projectListCollapsed ? '▼' : '▲' }}</span>
          <span class="toggle-title">📂 {{ t('active_projects') }} ({{ store.activeProjects.length }})</span>
        </div>

        <!-- 已打开项目列表 -->
        <ProjectList
          v-show="!compactMode && !projectListCollapsed"
          :projects="store.activeProjects"
          :providers="store.providers"
          @removed="handleRemoveProject"
          @launch="handleLaunchProject"
          @reorder="handleReorderProjects"
        />
      </div>
    </div>

    <!-- 设置页面 -->
    <Settings
      v-if="currentPage === 'settings'"
      @back="goBack"
      @openTemplates="openTemplates"
      @openSkills="openSkills"
      @openPlugins="openPlugins"
    />

    <!-- 编辑/添加供应商页面 -->
    <ProviderForm
      v-if="currentPage === 'form'"
      :provider="editingProvider"
      @back="goBack"
      @done="goBack(); store.loadProviders()"
    />

    <!-- 模板管理页面 -->
    <TemplateManager
      v-if="currentPage === 'templates'"
      @back="goBack"
    />

    <!-- Skill 管理页面 -->
    <SkillManager
      v-if="currentPage === 'skills'"
      @back="goBack"
    />

    <!-- 插件管理页面 -->
    <PluginManager
      v-if="currentPage === 'plugins'"
      @back="goBack"
    />

    <!-- Token 统计页面 -->
    <TokenStats
      v-if="currentPage === 'usage-stats'"
      @back="goBack"
    />
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}
.page-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.content {
  flex: 1;
  min-height: 0;
  padding: 8px 16px 0;
  display: flex;
  flex-direction: column;
}

/* 项目列表折叠控制 */
.project-section-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 0;
  margin-top: 12px;
  border-top: 1px solid #e8e8e8;
  cursor: pointer;
  user-select: none;
  transition: background 0.15s;
  flex-shrink: 0;
}
.project-section-toggle:hover {
  background: rgba(128,128,128,0.05);
}
.toggle-icon {
  font-size: 12px;
  color: #999;
  width: 20px;
  text-align: center;
}
.toggle-title {
  font-size: 13px;
  font-weight: 600;
  color: #555;
}
body.dark .project-section-toggle { border-top-color: #333; }
body.dark .toggle-title { color: #aaa; }

/* 简洁扁平工具栏（参考 LobeHub 风格） */
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  border-bottom: 1px solid #eee;
  background: #fafafa;
  flex-shrink: 0;
}
.toolbar-btn {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s;
  background: transparent;
  user-select: none;
}
.toolbar-btn:hover {
  background: rgba(0,0,0,0.06);
}
.toolbar-btn svg {
  color: #555;
}

/* 深色模式适配 */
body.dark .toolbar {
  border-bottom-color: #333;
  background: #242424;
}
body.dark .toolbar-btn:hover {
  background: rgba(255,255,255,0.08);
}
body.dark .toolbar-btn svg {
  color: #aaa;
}

/* 响应式：窄屏时保持紧凑 */
@media (max-width: 400px) {
  .toolbar { gap: 4px; padding: 6px 8px; }
  .toolbar-btn { width: 32px; height: 32px; }
}
</style>
