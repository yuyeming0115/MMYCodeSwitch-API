<script setup lang="ts">
import { ref, onMounted, computed, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessage, useDialog } from 'naive-ui'
import { useAppStore, type Provider } from '../stores/app'
import { i18n } from '../i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import ProviderGrid from './ProviderGrid.vue'
import ProviderForm from './ProviderForm.vue'
import Settings from './Settings.vue'
import ProjectList from './ProjectList.vue'
import TemplateManager from './TemplateManager.vue'
import SkillManager from './SkillManager.vue'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

// 当前页面
const currentPage = ref<'main' | 'settings' | 'form' | 'templates' | 'skills'>('main')
const editingProvider = ref<Provider | undefined>()
const isDark = defineModel<boolean>('isDark', { default: false })

const appWindow = getCurrentWindow()
let resizeTimeout: ReturnType<typeof setTimeout> | null = null

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

  // 恢复深浅色模式和精简模式状态
  try {
    const saved = await invoke<{ isDark?: boolean; compactMode?: boolean } | null>('get_window_state') as any
    if (saved && typeof saved.isDark === 'boolean') {
      isDark.value = saved.isDark
    }
    if (saved && typeof saved.compactMode === 'boolean') {
      compactMode.value = saved.compactMode
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
})

onUnmounted(() => {
  if (resizeTimeout) clearTimeout(resizeTimeout)
})

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

// 监听精简模式变化，自动保存
watch(compactMode, async (val) => {
  try {
    const pos = await appWindow.outerPosition()
    const size = await appWindow.outerSize()
    await invoke('save_window_state', { x: pos.x, y: pos.y, width: size.width, height: size.height, isDark: isDark.value, compactMode: val })
  } catch (_) {}
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

/// 一键清理所有未运行 Claude CLI 的项目
async function handleCleanupProjects() {
  if (store.activeProjects.length === 0) {
    msg.info(t('cleanup_none'))
    return
  }

  const loadingMsg = msg.loading(t('cleanup_checking'), { duration: 0 })

  try {
    // 1. 获取所有项目路径
    const projectPaths = store.activeProjects.map(p => p.project_path)

    // 2. 调用后端检测哪些项目有活跃的 Claude CLI
    const activeProcesses = await invoke<{ project_path: string; pid: number }[]>('check_active_claude_processes', { projectPaths })

    loadingMsg.destroy()

    // 3. 找出活跃项目路径
    const activePaths = new Set(activeProcesses.map(p => normalizePath(p.project_path)))

    // 4. 找出待清理项目（不在活跃列表中的）
    const toRemove = store.activeProjects.filter(proj => !activePaths.has(normalizePath(proj.project_path)))

    if (toRemove.length === 0) {
      msg.success(t('cleanup_none'))
      return
    }

    // 5. 弹出确认框
    dialog.warning({
      title: t('cleanup_confirm_title'),
      content: t('cleanup_confirm_msg', { inactive: toRemove.length, active: activePaths.size }),
      positiveText: t('confirm'),
      negativeText: t('cancel'),
      onPositiveClick: async () => {
        // 6. 执行清理
        for (const proj of toRemove) {
          await store.removeActiveProject(proj.id)
        }
        msg.success(t('cleanup_success', { n: toRemove.length }))
      },
    })
  } catch (e) {
    loadingMsg.destroy()
    console.error('[handleCleanupProjects] 检测失败:', e)
    msg.error('检测失败: ' + (e instanceof Error ? e.message : String(e)))
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

// 状态栏信息
const statusInfo = computed(() => t('right_click_hint'))
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
      <div class="content">
        <ProviderGrid
          :providers="store.providers"
          :active-provider-id="activeProviderId"
          @switch="doSwitch"
          @edit="openEdit"
          @delete="confirmDelete"
          @add="openAdd"
        />

        <!-- 已打开项目折叠控制 -->
        <div v-show="!compactMode" class="project-section-toggle" @click="projectListCollapsed = !projectListCollapsed">
          <span class="toggle-icon">{{ projectListCollapsed ? '▼' : '▲' }}</span>
          <span class="toggle-title">📂 {{ t('active_projects') }} ({{ store.activeProjects.length }})</span>
        </div>

        <!-- 已打开项目列表 -->
        <ProjectList
          v-show="!compactMode"
          :projects="store.activeProjects"
          :providers="store.providers"
          @removed="handleRemoveProject"
          @launch="handleLaunchProject"
        />
      </div>

      <footer v-show="!compactMode" class="toolbar">
        <n-button size="large" secondary @click="currentPage = 'templates'">📝 {{ t('templates') }}</n-button>
        <n-button size="large" secondary @click="currentPage = 'skills'">🔧 {{ t('skills') }}</n-button>
        <n-button size="large" secondary @click="isDark = !isDark" class="always-visible">{{ isDark ? '☀️' : '🌙' }}</n-button>
        <n-button size="large" secondary @click="currentPage = 'settings'" class="always-visible">⚙️</n-button>
      </footer>

      <footer v-show="!compactMode" class="statusbar">
        <span class="statusbar-left">{{ statusInfo }}</span>
        <n-button
          v-if="store.activeProjects.length > 0"
          size="tiny"
          type="error"
          secondary
          class="statusbar-right"
          @click="handleCleanupProjects"
        >🧹 {{ t('cleanup_all_projects') }}</n-button>
      </footer>
    </div>

    <!-- 设置页面 -->
    <Settings
      v-if="currentPage === 'settings'"
      @back="goBack"
      @openTemplates="openTemplates"
      @openSkills="openSkills"
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
  display: grid;
  grid-template-rows: 1fr auto auto;
  flex: 1;
  min-height: 0;
}
.content {
  min-height: 0; /* 关键：允许 grid 子项收缩 */
  overflow-y: auto;
  padding: 8px 16px 0;
  /* 全局美化滚动条 */
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.25) transparent;
}
.content::-webkit-scrollbar { width: 5px; }
.content::-webkit-scrollbar-track { background: transparent; }
.content::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.25);
  border-radius: 10px;
}
.content::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.45);
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

.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 16px;
  border-top: 1px solid #eee;
  flex-shrink: 0;
  background: #fafafa;
  min-height: 52px;
}
.toolbar-left {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  flex: 1;
  justify-content: center;
}
.toolbar-right {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}
.statusbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px;
  font-size: 11px;
  color: #888;
  background: #f5f5f5;
  flex-shrink: 0;
  gap: 8px;
  min-height: 32px;
}
.statusbar-left {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.statusbar-right {
  flex-shrink: 0;
}
.always-visible {
  flex-shrink: 0;
  order: 100;  /* 放到最后，确保始终可见 */
}

/* 深色模式适配 */
body.dark .toolbar {
  border-top-color: #333;
  background: #242424;
}
body.dark .statusbar {
  color: #888;
  background: #1a1a1a;
}
body.dark .content { scrollbar-color: rgba(200,200,200,0.12) transparent; }
body.dark .content::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.12); }
body.dark .content::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.28); }

/* 响应式：窄屏时按钮变小 */
@media (max-width: 400px) {
  .toolbar { gap: 6px; padding: 8px 12px; }
  .toolbar .n-button { font-size: 12px; }
}
</style>
