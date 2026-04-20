<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
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
import QuickSetup from './QuickSetup.vue'
import ProjectList from './ProjectList.vue'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

// 当前页面：'main' | 'quickSetup' | 'settings' | 'form'
const currentPage = ref<'main' | 'quickSetup' | 'settings' | 'form'>('main')
const editingProvider = ref<Provider | undefined>()
const isDark = defineModel<boolean>('isDark', { default: false })

onMounted(async () => {
  await store.init()
  i18n.global.locale.value = store.config.language as 'zh' | 'en'

  // 恢复深浅色模式状态
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const saved = await invoke<{ isDark?: boolean } | null>('get_window_state') as any
    if (saved && typeof saved.isDark === 'boolean') {
      isDark.value = saved.isDark
    }
  } catch (_) { /* 首次运行无状态文件 */ }
})

// 监听深浅模式变化，自动保存
watch(isDark, async (val) => {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const pos = await appWindow.outerPosition()
    const size = await appWindow.outerSize()
    await invoke('save_window_state', { x: pos.x, y: pos.y, width: size.width, height: size.height, isDark: val })
  } catch (_) {}
})

const activeInstance = computed(() => store.activeInstance())
const activeProviderId = computed(() => activeInstance.value?.active_provider_id)
const appWindow = getCurrentWindow()
const isMaxed = ref(false)
const injecting = ref(false)  // 注入进行中，防止重复点击

async function toggleMax() {
  if (await appWindow.isMaximized()) {
    await appWindow.unmaximize()
  } else {
    await appWindow.maximize()
  }
}

async function doCloseWindow() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
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
  return path.replace(/\\/g, '/').trimEnd('/')
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
  <div class="app">
    <!-- 自定义标题栏 -->
    <div class="titlebar">
      <span class="titlebar-title">MMYCodeSwitch-API</span>
      <div class="titlebar-controls">
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

        <!-- 已打开项目列表 -->
        <ProjectList
          :projects="store.activeProjects"
          :providers="store.providers"
          @removed="handleRemoveProject"
          @launch="handleLaunchProject"
        />
      </div>

      <footer class="toolbar">
        <n-button type="primary" size="large" @click="currentPage = 'quickSetup'">⚡ {{ t('quick_setup') }}</n-button>
        <n-button size="large" secondary @click="isDark = !isDark">{{ isDark ? '☀️' : '🌙' }}</n-button>
        <n-button size="large" secondary @click="currentPage = 'settings'">⚙️</n-button>
      </footer>

      <footer class="statusbar">
        <span>{{ statusInfo }}</span>
      </footer>
    </div>

    <!-- 快速配置页面 -->
    <QuickSetup
      v-if="currentPage === 'quickSetup'"
      @back="goBack"
      @done="goBack(); store.loadProviders()"
    />

    <!-- 设置页面 -->
    <Settings
      v-if="currentPage === 'settings'"
      @back="goBack"
    />

    <!-- 编辑/添加供应商页面 -->
    <ProviderForm
      v-if="currentPage === 'form'"
      :provider="editingProvider"
      @back="goBack"
      @done="goBack(); store.loadProviders()"
    />
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.page-main {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.content {
  flex: 1;
  overflow-y: auto;
  padding: 8px 16px 0;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 12px 16px;
  border-top: 1px solid #eee;
  flex-shrink: 0;
  background: #fafafa;
}
.statusbar {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px 16px;
  font-size: 11px;
  color: #888;
  background: #f5f5f5;
  flex-shrink: 0;
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
</style>
