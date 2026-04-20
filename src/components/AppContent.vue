<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessage, useDialog } from 'naive-ui'
import { useAppStore, type Provider } from '../stores/app'
import { i18n } from '../i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import ProviderGrid from './ProviderGrid.vue'
import ProviderForm from './ProviderForm.vue'
import Settings from './Settings.vue'
import QuickSetup from './QuickSetup.vue'

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

async function doSwitch(p: Provider) {
  try {
    const inst = store.activeInstance()
    const targetDir = inst?.config_dir
    if (!targetDir) {
      msg.error('未找到目标实例')
      return
    }

    // 单实例模式：直接切换
    if (store.config.instances.length <= 1) {
      await store.switchProvider(p.id)
      msg.success(`✅ 已切换到「${p.name}」`, { duration: 3000 })
      return
    }

    // 多实例模式：弹出确认对话框
    dialog.info({
      title: '确认切换',
      content: () => {
        const h = (window as any).Vue?.h || (() => null)
        return [
          `即将把「${p.name}」的配置注入到：`,
          `\n📁 ${inst?.name || targetDir}`,
          '\n\n请确认是否继续？',
        ].join('')
      },
      positiveText: '确认切换',
      negativeText: '取消',
      onPositiveClick: async () => {
        await store.switchProvider(p.id)
        msg.success(`✅ 已将「${p.name}」注入到「${inst?.name || targetDir}」`, { duration: 4000 })
      },
    })
  } catch (e) {
    msg.error(t('switch_failed') + ': ' + e)
  }
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
