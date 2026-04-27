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

        <!-- 模板和 Skill 入口 -->
        <n-divider>{{ t('templates') }} & {{ t('skills') }}</n-divider>
        <n-space>
          <n-button @click="emit('openTemplates')">{{ t('templates') }} ({{ store.templates.length }})</n-button>
          <n-button @click="emit('openSkills')">{{ t('skills') }} ({{ store.skills.length }})</n-button>
        </n-space>

        <n-divider>{{ t('export_backup') }} / {{ t('import_backup') }}</n-divider>

        <!-- 完整备份选项 -->
        <n-form-item :label="t('full_backup_include_templates')">
          <n-checkbox v-model:checked="includeTemplates" />
        </n-form-item>
        <n-form-item :label="t('full_backup_include_skills')">
          <n-checkbox v-model:checked="includeSkills" />
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

        <!-- 导出按钮 -->
        <n-form-item :label="t('export_backup')">
          <n-button type="primary" @click="doExport">{{ t('export_backup') }}</n-button>
        </n-form-item>

        <!-- 导入区域 -->
        <n-form-item :label="t('import_backup')">
          <n-space align="center">
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
import { open as dialogOpen } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { useAppStore } from '../stores/app'
import { useMessage } from 'naive-ui'
import { i18n } from '../i18n'
import { FolderOutline as folderOutlineIcon } from '@vicons/ionicons5'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const emit = defineEmits<{ back: [], openTemplates: [], openSkills: [] }>()

const usePassword = ref(false)
const backupPassword = ref('')
const defaultDir = ref(store.config.defaultConfigDir || '')
const exportPath = ref(store.config.backupExportPath || '')
const includeTemplates = ref(true)
const includeSkills = ref(true)

const importStatus = ref<{ type: 'info' | 'warning' | 'success', text: string } | null>(null)
const showPasswordModal = ref(false)
const importPassword = ref('')
const pendingBackupData = ref<number[] | null>(null)

function setLang(lang: string) {
  store.config.language = lang
  i18n.global.locale.value = lang as 'zh' | 'en'
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
      customPath: exportPath.value || null
    })
    msg.success(t('backup_export_quick_success', { path: result.path }))
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
        const result = await invoke<{ providers_count: number, templates_count: number, skills_count: number }>('import_full_backup', {
          data,
          password: '',
          importTemplates: true,
          importSkills: true
        })
        await store.loadProviders()
        await store.loadTemplates()
        await store.loadSkills()
        await store.loadTemplateBindings()
        importStatus.value = { type: 'success', text: t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count }) }
        msg.success(t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count }))
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
    const result = await invoke<{ providers_count: number, templates_count: number, skills_count: number }>('import_full_backup', {
      data,
      password: importPassword.value,
      importTemplates: true,
      importSkills: true
    })
    await store.loadProviders()
    await store.loadTemplates()
    await store.loadSkills()
    await store.loadTemplateBindings()
    importStatus.value = { type: 'success', text: t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count }) }
    msg.success(t('full_backup_import_success', { providers: result.providers_count, templates: result.templates_count, skills: result.skills_count }))
  } catch (e) {
    msg.error(String(e))
  }

  pendingBackupData.value = null
  importPassword.value = ''
}

async function save() {
  store.config.defaultConfigDir = defaultDir.value || undefined
  store.config.backupExportPath = exportPath.value || undefined
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
.page-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  padding-bottom: 80px;  /* 为底部按钮预留空间 */
  /* 继承全局滚动条样式 */
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.2) transparent;
}
.page-content::-webkit-scrollbar { width: 6px; }
.page-content::-webkit-scrollbar-track { background: transparent; }
.page-content::-webkit-scrollbar-thumb { background: rgba(128,128,128,0.2); border-radius: 10px; }
.page-content::-webkit-scrollbar-thumb:hover { background: rgba(128,128,128,0.4); }
body.dark .page-content { scrollbar-color: rgba(200,200,200,0.12) transparent; }
body.dark .page-content::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.12); }
body.dark .page-content::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.25); }
.page-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-top: 1px solid #eee;
  background: #fafafa;
  flex-shrink: 0;
  position: sticky;
  bottom: 0;
  z-index: 10;
}
body.dark .page-footer { background: #242424; border-top-color: #333; }
.hint-text { color: #666; font-size: 12px; margin-bottom: 8px; }
body.dark .hint-text { color: #999; }
.import-status { font-size: 12px; padding: 2px 8px; border-radius: 4px; }
.import-status.info { background: #e8f4ff; color: #2080f0; }
.import-status.warning { background: #fff7e6; color: #f0a020; }
.import-status.success { background: #e8f5e9; color: #18a058; }
body.dark .import-status.info { background: #2a3a5a; color: #70c0e8; }
body.dark .import-status.warning { background: #3a3520; color: #f0a020; }
body.dark .import-status.success { background: #2a3a30; color: #18a058; }
</style>