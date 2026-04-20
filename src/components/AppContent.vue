<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessage, useDialog } from 'naive-ui'
import { useAppStore, type Provider } from '../stores/app'
import { i18n } from '../i18n'
import ProviderGrid from './ProviderGrid.vue'
import ProviderForm from './ProviderForm.vue'
import Settings from './Settings.vue'
import QuickSetup from './QuickSetup.vue'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

const showForm = ref(false)
const showSettings = ref(false)
const showQuickSetup = ref(false)
const editingProvider = ref<Provider | undefined>()
const isDark = defineModel<boolean>('isDark', { default: false })

onMounted(async () => {
  await store.init()
  i18n.global.locale.value = store.config.language as 'zh' | 'en'
})

const activeInstance = computed(() => store.activeInstance())
const activeProviderId = computed(() => activeInstance.value?.active_provider_id)

function openAdd() {
  editingProvider.value = undefined
  showForm.value = true
}

function openEdit(p: Provider) {
  editingProvider.value = p
  showForm.value = true
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
        // 使用 h 创建 VNode 来动态展示信息
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
</script>

<template>
  <div class="app">
    <!-- 主内容区：仅 ProviderGrid -->
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

    <!-- 底部工具栏：快速配置 + 主题切换 + 设置 -->
    <footer class="toolbar">
      <n-button type="primary" size="large" @click="showQuickSetup = true">⚡ {{ t('quick_setup') }}</n-button>
      <n-button size="large" secondary @click="isDark = !isDark">{{ isDark ? '☀️' : '🌙' }}</n-button>
      <n-button size="large" secondary @click="showSettings = true">⚙️</n-button>
    </footer>

    <!-- 底部状态栏：显示操作提示 -->
    <footer class="statusbar">
      <span>{{ t('right_click_hint') }}</span>
    </footer>

    <ProviderForm v-model:show="showForm" :provider="editingProvider" @done="store.loadProviders" />
    <Settings v-model:show="showSettings" />
    <QuickSetup v-model:show="showQuickSetup" @done="store.loadProviders" />
  </div>
</template>

<style scoped>
.app {
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
