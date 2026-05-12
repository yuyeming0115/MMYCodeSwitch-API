<template>
  <div class="page">
    <div class="page-content">
      <n-form label-placement="left" label-width="80px">
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

        <!-- 模板和 Skill 入口 -->
        <n-divider>{{ t('templates') }} / {{ t('skills') }} / {{ t('plugins') }}</n-divider>
        <n-space>
          <n-button @click="emit('openTemplates')">{{ t('templates') }} ({{ store.templates.length }})</n-button>
          <n-button @click="emit('openSkills')">{{ t('skills') }} ({{ store.skills.length }})</n-button>
          <n-button @click="emit('openPlugins')">{{ t('plugins') }} ({{ store.plugins.length }})</n-button>
        </n-space>

        <n-divider>{{ t('export_backup') }} / {{ t('import_backup') }}</n-divider>

        <!-- 完整备份选项 -->
        <n-form-item label="包含内容">
          <n-space align="center">
            <n-checkbox v-model:checked="includeTemplates">{{ t('templates') }}</n-checkbox>
            <n-checkbox v-model:checked="includeSkills">{{ t('skills') }}</n-checkbox>
            <n-checkbox v-model:checked="includePlugins">{{ t('plugins') }}</n-checkbox>
            <span v-if="includePlugins" class="form-hint">{{ t('full_backup_plugin_hint') }}</span>
          </n-space>
        </n-form-item>

        <!-- 导出路径 -->
        <n-form-item :label="t('backup_export_path')">
          <n-space align="center" style="width: 100%">
            <n-input
              v-model:value="exportPath"
              style="width: 220px"
              :placeholder="t('backup_export_path_hint')"
            />
            <n-button dashed @click="browseExportPath">
              <template #icon><n-icon><folder-outline-icon /></n-icon></template>
            </n-button>
            <n-button v-if="exportPath" dashed size="small" @click="clearExportPath">清除</n-button>
          </n-space>
        </n-form-item>

        <!-- 密码设置 -->
        <n-form-item :label="t('backup_password_optional')">
          <n-space align="center">
            <n-checkbox v-model:checked="usePassword">
              {{ t('backup_set_password') }}
            </n-checkbox>
            <n-input
              v-if="usePassword"
              v-model:value="backupPassword"
              type="password"
              show-password-on="click"
              style="width:180px"
              :placeholder="t('backup_password_label')"
            />
          </n-space>
        </n-form-item>

        <!-- 导出/导入按钮 -->
        <n-form-item label="操作">
          <n-space align="center">
            <n-button type="primary" @click="doExport">{{ t('export_backup') }}</n-button>
            <n-button @click="doImport">{{ t('import_backup') }}</n-button>
            <span v-if="importStatus" class="import-status" :class="importStatus.type">
              {{ importStatus.text }}
            </span>
          </n-space>
        </n-form-item>

        <!-- 导入密码输入 -->
        <n-modal v-model:show="showPasswordModal" preset="dialog" :title="t('backup_import_need_pwd')">
          <n-form-item :label="t('backup_password_label')">
            <n-input v-model:value="importPassword" type="password" show-password-on="click" style="width:200px" />
          </n-form-item>
          <template #action>
            <n-button @click="showPasswordModal = false">{{ t('cancel') }}</n-button>
            <n-button type="primary" @click="doImportWithPassword">{{ t('confirm') }}</n-button>
          </template>
        </n-modal>

        <!-- 备份文件列表 -->
        <n-divider>{{ t('backup_files') }}</n-divider>
        <p class="hint-text">{{ t('backup_files_hint') }}</p>
        <div v-if="backupFiles.length > 0" class="backup-list">
          <div v-for="file in backupFiles" :key="file.path" class="backup-item">
            <div class="backup-info">
              <span class="backup-name">{{ file.filename }}</span>
              <span class="backup-meta">{{ file.created_at }} · {{ formatSize(file.size) }}</span>
            </div>
            <div class="backup-actions">
              <div class="backup-btn restore" @click="doRestoreBackup(file)" title="恢复">
                <span class="btn-icon">↻</span>
              </div>
              <div class="backup-btn delete" @click="confirmDeleteBackup(file)" title="删除">
                <span class="btn-icon">🗑</span>
              </div>
            </div>
          </div>
        </div>
        <p v-else class="no-backup">{{ t('backup_no_files') }}</p>

        <!-- 恢复备份密码输入 -->
        <n-modal v-model:show="showRestorePasswordModal" preset="dialog" :title="t('backup_import_need_pwd')">
          <n-form-item :label="t('backup_password_label')">
            <n-input v-model:value="restorePassword" type="password" show-password-on="click" style="width:200px" />
          </n-form-item>
          <template #action>
            <n-button @click="showRestorePasswordModal = false">{{ t('cancel') }}</n-button>
            <n-button type="primary" @click="doRestoreWithPassword">{{ t('confirm') }}</n-button>
          </template>
        </n-modal>
      </n-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open as dialogOpen } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { useAppStore, type BackupFile } from '../stores/app'
import { useMessage, useDialog } from 'naive-ui'
import { FolderOutline as folderOutlineIcon } from '@vicons/ionicons5'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()
const emit = defineEmits<{ openTemplates: [], openSkills: [], openPlugins: [] }>()

const usePassword = ref(false)
const backupPassword = ref('')
const defaultDir = ref(store.config.defaultConfigDir || '')
const exportPath = ref(store.config.backupExportPath || '')
const includeTemplates = ref(true)
const includeSkills = ref(true)
const includePlugins = ref(true)

const importStatus = ref<{ type: 'info' | 'warning' | 'success', text: string } | null>(null)
const showPasswordModal = ref(false)
const importPassword = ref('')
const pendingBackupData = ref<number[] | null>(null)

// 备份文件列表
const backupFiles = ref<BackupFile[]>([])

// 恢复备份
const showRestorePasswordModal = ref(false)
const restorePassword = ref('')
const pendingRestoreFile = ref<BackupFile | null>(null)

onMounted(async () => {
  await loadBackupFiles()
})

async function loadBackupFiles() {
  try {
    backupFiles.value = await store.loadBackupFiles()
  } catch (e) {
    console.error('[loadBackupFiles] 错误:', e)
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function confirmDeleteBackup(file: BackupFile) {
  dialog.warning({
    title: t('backup_delete_confirm_title'),
    content: file.filename,
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: async () => {
      try {
        await store.deleteBackupFile(file.path)
        await loadBackupFiles()
        msg.success(t('backup_delete_success'))
      } catch (e) {
        msg.error(String(e))
      }
    },
  })
}

// 从备份文件恢复
async function doRestoreBackup(file: BackupFile) {
  try {
    const fileData = await readFile(file.path)
    const data: number[] = Array.from(fileData)

    // 检查文件格式
    if (data.slice(0, 5).map(b => String.fromCharCode(b)).join('') !== 'MMYCS') {
      msg.error('无法识别的备份文件格式')
      return
    }

    const info = await invoke<{ version: number, same_machine: boolean, has_password: boolean }>('check_full_backup_file', { data })

    if (info.same_machine && !info.has_password) {
      // 本机备份，无需密码，直接恢复
      await doRestore(data, '')
    } else if (info.has_password) {
      // 需要密码
      pendingRestoreFile.value = file
      showRestorePasswordModal.value = true
    } else {
      msg.warning(t('backup_import_no_pwd_cross'))
    }
  } catch (e) {
    msg.error('读取备份失败: ' + String(e))
  }
}

async function doRestoreWithPassword() {
  if (!restorePassword.value) {
    msg.warning(t('backup_password_hint'))
    return
  }

  showRestorePasswordModal.value = false

  if (!pendingRestoreFile.value) return

  try {
    const fileData = await readFile(pendingRestoreFile.value.path)
    const data: number[] = Array.from(fileData)
    await doRestore(data, restorePassword.value)
  } catch (e) {
    msg.error('恢复失败: ' + String(e))
  }

  pendingRestoreFile.value = null
  restorePassword.value = ''
}

async function doRestore(data: number[], password: string) {
  const loadingMsg = msg.loading(t('backup_restoring'), { duration: 0 })
  try {
    const result = await invoke<{ providers_count: number, templates_count: number, skills_count: number, plugins_count: number }>('import_full_backup', {
      data,
      password,
      importTemplates: true,
      importSkills: true,
      importPlugins: true
    })
    await store.loadProviders()
    await store.loadTemplates()
    await store.loadSkills()
    await store.loadTemplateBindings()
    await store.loadPlugins()
    await store.loadMarketplaces()
    loadingMsg.destroy()
    msg.success(t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count, plugins: result.plugins_count }))
  } catch (e) {
    loadingMsg.destroy()
    msg.error('恢复失败: ' + String(e))
  }
}

async function browseDefaultDir() {
  try {
    const selected = await dialogOpen({ directory: true, title: t('select_default_dir') })
    if (selected) {
      defaultDir.value = selected as string
    }
  } catch (e) {
    console.error('[browseDefaultDir] 错误:', e)
    msg.error(t('browse_failed') + ': ' + (e instanceof Error ? e.message : String(e)))
  }
}

async function browseExportPath() {
  try {
    const selected = await dialogOpen({ directory: true, title: t('backup_export_path') })
    if (selected) {
      exportPath.value = selected as string
      // 立即保存路径到 config（自动记忆）
      store.config.backupExportPath = selected as string
      await store.saveConfig(store.config)
      msg.info(t('backup_path_remembered'))
    }
  } catch (e) {
    console.error('[browseExportPath] 错误:', e)
    msg.error(t('browse_failed') + ': ' + (e instanceof Error ? e.message : String(e)))
  }
}

function clearExportPath() {
  exportPath.value = ''
  // 清除保存的路径
  store.config.backupExportPath = undefined
  store.saveConfig(store.config)
}

// 完整导出
async function doExport() {
  try {
    const password = usePassword.value && backupPassword.value ? backupPassword.value : ''
    const result = await invoke<{ path: string, filename: string, included: string[] }>('export_full_backup', {
      password,
      includeTemplates: includeTemplates.value,
      includeSkills: includeSkills.value,
      includePlugins: includePlugins.value,
      customPath: exportPath.value || null
    })
    msg.success(t('backup_export_quick_success', { path: result.path }))
    // 刷新备份文件列表
    await loadBackupFiles()
  } catch (e) {
    msg.error('导出失败: ' + String(e))
  }
}

// 完整导入
async function doImport() {
  importStatus.value = null

  const path = await dialogOpen({
    filters: [{ name: 'MMYCS Backup', extensions: ['mmycs'] }]
  })
  if (!path) return

  const fileData = await readFile(path as string)
  const data: number[] = Array.from(fileData)

  if (data.slice(0, 5).map(b => String.fromCharCode(b)).join('') === 'MMYCS') {
    const info = await invoke<{ version: number, same_machine: boolean, has_password: boolean }>('check_full_backup_file', { data })

    if (info.same_machine && !info.has_password) {
      importStatus.value = { type: 'info', text: t('backup_same_machine') }
      try {
        const result = await invoke<{ providers_count: number, templates_count: number, skills_count: number, plugins_count: number }>('import_full_backup', {
          data,
          password: '',
          importTemplates: true,
          importSkills: true,
          importPlugins: true
        })
        await store.loadProviders()
        await store.loadTemplates()
        await store.loadSkills()
        await store.loadTemplateBindings()
        await store.loadPlugins()
        await store.loadMarketplaces()
        importStatus.value = { type: 'success', text: t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count, plugins: result.plugins_count }) }
        msg.success(t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count, plugins: result.plugins_count }))
      } catch (e) {
        msg.error(String(e))
      }
    } else if (info.has_password) {
      importStatus.value = { type: 'warning', text: t('backup_import_need_pwd') }
      pendingBackupData.value = data
      showPasswordModal.value = true
    } else {
      importStatus.value = { type: 'warning', text: t('backup_import_no_pwd_cross') }
      msg.warning(t('backup_import_no_pwd_cross'))
    }
  } else {
    msg.error('无法识别的备份文件格式')
  }
}

async function doImportWithPassword() {
  if (!importPassword.value) {
    msg.warning(t('backup_password_hint'))
    return
  }

  showPasswordModal.value = false

  if (!pendingBackupData.value) return

  const data = pendingBackupData.value

  try {
    const result = await invoke<{ providers_count: number, templates_count: number, skills_count: number, plugins_count: number }>('import_full_backup', {
      data,
      password: importPassword.value,
      importTemplates: true,
      importSkills: true,
      importPlugins: true
    })
    await store.loadProviders()
    await store.loadTemplates()
    await store.loadSkills()
    await store.loadTemplateBindings()
    await store.loadPlugins()
    await store.loadMarketplaces()
    importStatus.value = { type: 'success', text: t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count, plugins: result.plugins_count }) }
    msg.success(t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count, plugins: result.plugins_count }))
  } catch (e) {
    msg.error(String(e))
  }

  pendingBackupData.value = null
  importPassword.value = ''
}

// 自动保存默认目录和导出路径
let settingsSaveTimer: ReturnType<typeof setTimeout> | null = null
watch([defaultDir, exportPath], () => {
  if (settingsSaveTimer) clearTimeout(settingsSaveTimer)
  settingsSaveTimer = setTimeout(async () => {
    store.config.defaultConfigDir = defaultDir.value || undefined
    store.config.backupExportPath = exportPath.value || undefined
    await store.saveConfig(store.config)
  }, 500)
})
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
.import-status { font-size: 12px; padding: 2px 8px; border-radius: 4px; }
.import-status.info { background: #e8f4ff; color: #2080f0; }
.import-status.warning { background: #fff7e6; color: #f0a020; }
.import-status.success { background: #e8f5e9; color: #18a058; }
body.dark .import-status.info { background: #2a3a5a; color: #70c0e8; }
body.dark .import-status.warning { background: #3a3520; color: #f0a020; }
body.dark .import-status.success { background: #2a3a30; color: #18a058; }
.form-hint { color: #999; font-size: 11px; margin-left: 4px; }
body.dark .form-hint { color: #777; }
/* 备份文件列表 */
.backup-list {
  margin-top: 8px;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
}
body.dark .backup-list { border-color: #333; }
.backup-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid #f0f0f0;
}
.backup-item:last-child { border-bottom: none; }
body.dark .backup-item { border-bottom-color: #2a2a2a; }
.backup-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.backup-name {
  font-size: 13px;
  color: #333;
  font-weight: 500;
}
body.dark .backup-name { color: #ddd; }
.backup-meta {
  font-size: 11px;
  color: #999;
}
.backup-actions {
  display: flex;
  gap: 6px;
}
.backup-btn {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: 2px solid #e0e0e0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: border-color 0.2s, box-shadow 0.2s, background 0.2s;
  background: #fff;
  user-select: none;
}
.backup-btn:hover {
  box-shadow: 0 2px 6px rgba(0,0,0,0.1);
}
.backup-btn.restore:hover {
  border-color: #2080f0;
  background: #e8f4ff;
}
.backup-btn.delete:hover {
  border-color: #d03050;
  background: #fff0f0;
}
.btn-icon {
  font-size: 14px;
  line-height: 1;
}
body.dark .backup-btn {
  background: #2a2a2a;
  border-color: #444;
}
body.dark .backup-btn.restore:hover {
  background: #2a3a5a;
  border-color: #2080f0;
}
body.dark .backup-btn.delete:hover {
  background: #3a2020;
  border-color: #d03050;
}
.no-backup {
  color: #999;
  font-size: 12px;
  margin-top: 8px;
}
</style>