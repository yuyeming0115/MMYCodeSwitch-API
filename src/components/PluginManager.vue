<template>
  <div class="page">
    <div class="page-content">
      <p class="hint-text">{{ t('plugin_hint') }}</p>

      <!-- 插件列表 -->
      <n-divider>{{ t('plugins') }}</n-divider>
      <div v-if="store.plugins.length === 0" class="empty-hint">
        {{ t('plugin_no_plugins') }}
        <br />
        <span class="sub-hint">{{ t('plugin_download_hint') }}</span>
      </div>
      <div v-else class="plugin-list">
        <div v-for="plugin in store.plugins" class="plugin-item" :key="plugin.id">
          <div class="plugin-header">
            <div class="plugin-info">
              <span class="plugin-name">{{ plugin.name }}</span>
              <span class="plugin-meta">
                <span class="plugin-marketplace">{{ plugin.marketplace }}</span>
                <span v-if="plugin.version" class="plugin-version">v{{ plugin.version }}</span>
              </span>
            </div>
            <n-switch
              :value="plugin.enabled"
              @update:value="(val: boolean) => togglePlugin(plugin.id, val)"
            >
              <template #checked>{{ t('plugin_enabled') }}</template>
              <template #unchecked>{{ t('plugin_disabled') }}</template>
            </n-switch>
          </div>
          <div v-if="plugin.path" class="plugin-path">{{ plugin.path }}</div>
        </div>
      </div>

      <!-- Marketplace 源管理 -->
      <n-divider>{{ t('marketplaces') }}</n-divider>
      <p class="hint-text">{{ t('marketplace_hint') }}</p>
      <div v-if="store.marketplaces.length === 0" class="empty-hint">{{ t('marketplace_no_sources') }}</div>
      <div v-else class="marketplace-list">
        <div v-for="mp in store.marketplaces" class="marketplace-item" :key="mp.id">
          <div class="marketplace-info">
            <span class="marketplace-id">{{ mp.id }}</span>
            <span v-if="mp.repo" class="marketplace-repo">→ {{ mp.repo }}</span>
          </div>
          <n-button size="small" type="error" @click="confirmRemoveMarketplace(mp.id)">
            {{ t('marketplace_remove') }}
          </n-button>
        </div>
      </div>

      <n-button type="primary" @click="showAddMarketplaceModal = true" style="margin-top: 16px">
        {{ t('marketplace_add') }}
      </n-button>

      <!-- 添加 Marketplace 模态框 -->
      <n-modal v-model:show="showAddMarketplaceModal" preset="dialog" :title="t('marketplace_add')">
        <n-form-item :label="t('marketplace_id')">
          <n-input v-model:value="newMarketplaceId" placeholder="claude-hud" />
        </n-form-item>
        <n-form-item :label="t('marketplace_repo')">
          <n-input v-model:value="newMarketplaceRepo" :placeholder="t('marketplace_repo_placeholder')" />
        </n-form-item>
        <template #action>
          <n-button @click="closeMarketplaceModal">{{ t('cancel') }}</n-button>
          <n-button type="primary" @click="addMarketplace">{{ t('save') }}</n-button>
        </template>
      </n-modal>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../stores/app'
import { useMessage, useDialog } from 'naive-ui'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

const showAddMarketplaceModal = ref(false)
const newMarketplaceId = ref('')
const newMarketplaceRepo = ref('')

async function togglePlugin(id: string, enabled: boolean) {
  try {
    await store.togglePlugin(id, enabled)
    msg.success(t('plugin_toggle_success'))
  } catch (e) {
    msg.error(t('plugin_toggle_failed', { msg: e instanceof Error ? e.message : String(e) }))
  }
}

function confirmRemoveMarketplace(id: string) {
  dialog.warning({
    title: t('confirm'),
    content: t('marketplace_remove_confirm', { id }),
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: async () => {
      await store.removeMarketplace(id)
      msg.success(t('marketplace_remove_success'))
    }
  })
}

async function addMarketplace() {
  if (!newMarketplaceId.value.trim() || !newMarketplaceRepo.value.trim()) {
    msg.error(t('required'))
    return
  }
  await store.addMarketplace(newMarketplaceId.value.trim(), newMarketplaceRepo.value.trim())
  msg.success(t('marketplace_add_success'))
  closeMarketplaceModal()
}

function closeMarketplaceModal() {
  showAddMarketplaceModal.value = false
  newMarketplaceId.value = ''
  newMarketplaceRepo.value = ''
}
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.page-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  padding-bottom: 16px;
}
.hint-text { color: #666; font-size: 12px; margin-bottom: 8px; }
body.dark .hint-text { color: #999; }
.empty-hint { color: #999; font-size: 13px; padding: 8px 0; }
.sub-hint { color: #aaa; font-size: 11px; margin-top: 4px; display: block; }
.plugin-list, .marketplace-list { display: flex; flex-direction: column; gap: 8px; }
.plugin-item, .marketplace-item {
  padding: 12px;
  background: #f5f5f5;
  border-radius: 6px;
}
body.dark .plugin-item, body.dark .marketplace-item { background: #2a2a2a; }
.plugin-header { display: flex; align-items: center; justify-content: space-between; }
.plugin-info { display: flex; flex-direction: column; gap: 4px; }
.plugin-name { font-weight: 600; font-size: 14px; }
.plugin-meta { display: flex; gap: 8px; font-size: 12px; color: #888; }
.plugin-marketplace {
  background: #e8e8e8;
  padding: 2px 6px;
  border-radius: 3px;
}
body.dark .plugin-marketplace { background: #3a3a3a; }
.plugin-version { color: #18a058; }
.plugin-path {
  font-size: 11px;
  color: #aaa;
  margin-top: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.marketplace-info { display: flex; align-items: center; gap: 8px; }
.marketplace-id { font-weight: 600; }
.marketplace-repo { color: #666; font-size: 12px; }
body.dark .marketplace-repo { color: #888; }
.marketplace-item { display: flex; align-items: center; justify-content: space-between; }
</style>