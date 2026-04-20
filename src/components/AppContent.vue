<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useMessage, useDialog } from 'naive-ui'
import { useAppStore, type Provider } from '../stores/app'
import { i18n } from '../i18n'
import ProviderGrid from './ProviderGrid.vue'
import ProviderForm from './ProviderForm.vue'
import Settings from './Settings.vue'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

const showForm = ref(false)
const showSettings = ref(false)
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
    await store.switchProvider(p.id)
    msg.success(t('switch_success') + ': ' + p.name)
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
    <header class="topbar">
      <span class="title">{{ t('app_title') }}</span>
      <n-select
        v-if="store.config.instances.length > 1"
        :value="store.activeInstanceId"
        :options="store.config.instances.map(i => ({ label: i.name, value: i.id }))"
        style="width:160px"
        @update:value="v => store.activeInstanceId = v"
      />
      <n-space>
        <n-button text @click="isDark = !isDark">{{ isDark ? '☀' : '☾' }}</n-button>
        <n-button text @click="showSettings = true">⚙</n-button>
      </n-space>
    </header>

    <div class="active-bar">
      <span>{{ t('current_active') }}：</span>
      <strong>{{ store.providers.find(p => p.id === activeProviderId)?.name ?? t('none') }}</strong>
      <span v-if="activeProviderId" style="color:#18a058;margin-left:4px">✓</span>
    </div>

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

    <div style="font-size:11px;color:#aaa;padding:4px 16px">右键卡片可编辑/删除</div>

    <ProviderForm v-model:show="showForm" :provider="editingProvider" @done="store.loadProviders" />
    <Settings v-model:show="showSettings" />
  </div>
</template>
