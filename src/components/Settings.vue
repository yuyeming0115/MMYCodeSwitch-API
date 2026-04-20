<template>
  <div class="page">
    <header class="page-header">
      <n-button text size="large" @click="emit('back')">←</n-button>
      <span class="page-title">{{ t('settings') }}</span>
    </header>

    <div class="page-content">
      <n-form label-placement="left" label-width="80px">
        <n-form-item :label="t('language')">
          <n-radio-group :value="store.config.language" @update:value="setLang">
            <n-radio value="zh">中文</n-radio>
            <n-radio value="en">English</n-radio>
          </n-radio-group>
        </n-form-item>

        <n-divider>{{ t('global_default_dir') }}</n-divider>
        <p class="hint-text">{{ t('global_default_dir_hint') }}</p>
        <div style="margin-bottom:8px">
          <n-space align="center">
            <n-input
              v-model:value="defaultDir"
              style="width:280px"
              :placeholder="t('config_dir_placeholder')"
            />
            <n-button dashed @click="browseDefaultDir">{{ t('browse') }}</n-button>
          </n-space>
        </div>

        <n-divider>{{ t('export_backup') }} / {{ t('import_backup') }}</n-divider>
        <n-form-item :label="t('backup_password')">
          <n-input v-model:value="backupPassword" type="password" show-password-on="click" :placeholder="t('backup_password_hint')" />
        </n-form-item>
        <n-space>
          <n-button @click="doExport">{{ t('export_backup') }}</n-button>
          <n-button @click="doImport">{{ t('import_backup') }}</n-button>
        </n-space>
      </n-form>
    </div>

    <footer class="page-footer">
      <n-button size="large" @click="emit('back')">{{ t('cancel') }}</n-button>
      <n-button type="primary" size="large" @click="save">{{ t('save') }}</n-button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { save as dialogSave, open as dialogOpen } from '@tauri-apps/plugin-dialog'
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs'
import { useAppStore } from '../stores/app'
import { useMessage } from 'naive-ui'
import { i18n } from '../i18n'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const emit = defineEmits<{ back: [] }>()
const backupPassword = ref('')
const defaultDir = ref(store.config.default_config_dir || '')

function setLang(lang: string) {
  store.config.language = lang
  i18n.global.locale.value = lang as 'zh' | 'en'
}

async function browseDefaultDir() {
  const selected = await dialogOpen({ directory: true, title: t('select_default_dir') })
  if (selected) {
    defaultDir.value = selected
  }
}

async function doExport() {
  if (!backupPassword.value) { msg.error(t('backup_password_hint')); return }
  const json = await invoke<string>('export_providers', { password: backupPassword.value })
  const path = await dialogSave({ defaultPath: 'mmycs_backup.json', filters: [{ name: 'JSON', extensions: ['json'] }] })
  if (!path) return
  await writeTextFile(path, json)
  msg.success(t('export_success'))
}

async function doImport() {
  if (!backupPassword.value) { msg.error(t('backup_password_hint')); return }
  const path = await dialogOpen({ filters: [{ name: 'JSON', extensions: ['json'] }] })
  if (!path) return
  const json = await readTextFile(path as string)
  try {
    const n = await invoke<number>('import_providers', { json, password: backupPassword.value })
    await store.loadProviders()
    msg.success(t('import_success', { n }))
  } catch (e) {
    msg.error(t('import_failed', { msg: String(e) }))
  }
}

async function save() {
  store.config.default_config_dir = defaultDir.value || undefined
  await store.saveConfig(store.config)
  msg.success(t('save_success'))
  emit('back')
}
</script>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.page-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid #eee;
  background: #fff;
  flex-shrink: 0;
}
body.dark .page-header { background: #242424; border-bottom-color: #333; }
.page-title { font-size: 18px; font-weight: 700; }
.page-content { flex: 1; overflow-y: auto; padding: 16px; }
.page-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-top: 1px solid #eee;
  background: #fafafa;
  flex-shrink: 0;
}
body.dark .page-footer { background: #242424; border-top-color: #333; }
</style>