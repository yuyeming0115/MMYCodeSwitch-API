<template>
  <div class="page">
    <header class="page-header">
      <n-button text size="large" @click="emit('back')">←</n-button>
      <span class="page-title">{{ t('templates') }}</span>
    </header>

    <div class="page-content">
      <p class="hint-text">{{ t('template_default_hint') }}</p>

      <!-- 模板列表 -->
      <n-divider>{{ t('templates') }}</n-divider>
      <div v-if="store.templates.length === 0" class="empty-hint">{{ t('none') }}</div>
      <div v-else class="template-list">
        <div
          v-for="tpl in store.templates"
          class="template-item"
          :class="{ builtin: tpl.builtin }"
          :key="tpl.name"
          @click="!tpl.builtin && editTemplate(tpl)"
        >
          <div class="template-header">
            <span class="template-name">
              {{ tpl.name }}
              <span v-if="tpl.builtin" class="builtin-badge">{{ t('template_builtin') }}</span>
            </span>
            <n-space>
              <template v-if="tpl.builtin">
                <n-button v-if="isAdopted(tpl.name)" size="small" disabled type="success">{{ t('template_adopted') }}</n-button>
                <n-button v-else size="small" type="primary" @click.stop="doAdopt(tpl.name)">{{ t('template_adopt') }}</n-button>
              </template>
              <template v-else>
                <n-button size="small" type="error" @click.stop="confirmDelete(tpl.name)">{{ t('delete') }}</n-button>
              </template>
            </n-space>
          </div>
        </div>
      </div>

      <n-button type="primary" @click="showAddModal = true" style="margin-top: 16px">
        {{ t('template_add') }}
      </n-button>

      <!-- 项目绑定 -->
      <n-divider>{{ t('template_bindings') }}</n-divider>
      <div v-if="store.templateBindings.length === 0" class="empty-hint">{{ t('template_no_bindings') }}</div>
      <div v-else class="binding-list">
        <div v-for="binding in store.templateBindings" class="binding-item" :key="binding.project_path">
          <span class="binding-path">{{ binding.project_path }}</span>
          <span class="binding-template">→ {{ binding.template_name }}</span>
          <n-button size="tiny" type="error" @click="unbind(binding.project_path)">{{ t('template_unbind') }}</n-button>
        </div>
      </div>

      <!-- 添加/编辑模态框 — 单文本框一键导入 -->
      <n-modal v-model:show="showAddModal" preset="dialog" :title="editing ? t('template_edit') : t('template_add')">
        <!-- 名称（只读，自动从内容第一行 # 标题提取） -->
        <n-form-item :label="t('template_name')">
          <n-input v-model:value="templateName" :disabled="true" :placeholder="t('template_name_auto')" />
        </n-form-item>
        <!-- 文本框（粘贴内容 / 拖入 .md 文件） -->
        <div
          class="drop-zone"
          :class="{ 'drag-over': isDragOver }"
          @dragover.prevent="isDragOver = true"
          @dragleave="isDragOver = false"
          @drop.prevent="onFileDrop"
        >
          <n-input
            v-model:value="templateContent"
            type="textarea"
            :rows="12"
            :placeholder="t('template_content_placeholder')"
            @input="autoExtractName"
          />
          <div v-if="isDragOver" class="drop-overlay">{{ t('drop_file_hint') }}</div>
        </div>
        <template #action>
          <n-button @click="closeModal">{{ t('cancel') }}</n-button>
          <n-button type="primary" @click="saveTemplate">{{ t('save') }}</n-button>
        </template>
      </n-modal>
    </div>

    <footer class="page-footer">
      <n-button size="medium" @click="emit('back')">{{ t('cancel') }}</n-button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore, type Template } from '../stores/app'
import { useMessage, useDialog } from 'naive-ui'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()
const emit = defineEmits<{ back: [] }>()

const showAddModal = ref(false)
const templateName = ref('')
const templateContent = ref('')
const editing = ref(false)
const isDragOver = ref(false)

/** 从内容第一行自动提取名称（markdown # 标题） */
function autoExtractName() {
  const firstLine = templateContent.value.split('\n')[0]?.trim()
  const match = firstLine?.match(/^#\s+(.+)/)
  if (match && match[1]) {
    templateName.value = match[1].trim()
  }
}

function editTemplate(tpl: Template) {
  editing.value = true
  templateName.value = tpl.name
  templateContent.value = tpl.content
  showAddModal.value = true
}

async function saveTemplate() {
  if (!templateName.value.trim()) {
    msg.error(t('required'))
    return
  }
  await store.saveTemplate(templateName.value.trim(), templateContent.value)
  msg.success(t('template_save_success'))
  closeModal()
}

function closeModal() {
  showAddModal.value = false
  templateName.value = ''
  templateContent.value = ''
  editing.value = false
  isDragOver.value = false
}

/** 拖入 .md 文件：读取文件名作为名称，内容作为模板内容 */
async function onFileDrop(e: DragEvent) {
  isDragOver.value = false
  const file = e.dataTransfer?.files?.[0]
  if (!file) return
  if (!file.name.endsWith('.md') && !file.name.endsWith('.txt')) {
    msg.error(t('template_drop_invalid_file'))
    return
  }
  const reader = new FileReader()
  reader.onload = () => {
    templateContent.value = reader.result as string
    templateName.value = file.name.replace(/\.(md|txt)$/, '')
    autoExtractName()
  }
  reader.readAsText(file)
}

function confirmDelete(name: string) {
  dialog.warning({
    title: t('confirm'),
    content: t('template_delete_confirm', { name }),
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: async () => {
      await store.deleteTemplate(name)
      msg.success(t('template_delete_success'))
    }
  })
}

async function unbind(projectPath: string) {
  await store.unbindTemplate(projectPath)
  msg.success(t('template_unbind'))
}

/** 判断内置模板是否已被用户采用（用户目录下存在同名模板文件） */
function isAdopted(name: string): boolean {
  return store.templates.some(t => t.name === name && !t.builtin)
}

/** 采用内置模板 */
async function doAdopt(name: string) {
  try {
    await store.adoptTemplate(name)
    msg.success(t('template_adopt_success'))
  } catch (e) {
    msg.error(t('template_adopt_failed', { msg: (e instanceof Error ? e.message : String(e)) }))
  }
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
  padding-bottom: 80px;
}
.page-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 8px 12px;
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
.empty-hint { color: #999; font-size: 13px; padding: 8px 0; }
.template-list, .binding-list { display: flex; flex-direction: column; gap: 8px; }
.template-item, .binding-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: #f5f5f5;
  border-radius: 6px;
  cursor: default;
}
.template-item:not(.builtin) {
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
  border: 1px solid transparent;
}
.template-item:not(.builtin):hover {
  background: #f0f0f0;
  border-color: #18a058;
}
body.dark .template-item, body.dark .binding-item { background: #2a2a2a; }
body.dark .template-item:not(.builtin):hover {
  background: #333;
  border-color: #18a058;
}
.template-header { display: flex; align-items: center; justify-content: space-between; width: 100%; }
.template-name { font-weight: 600; }
.binding-path { font-size: 12px; color: #666; max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
body.dark .binding-path { color: #888; }
.binding-template { font-size: 12px; color: #2080f0; margin-left: 8px; }

/* 内置模板样式 */
.template-item.builtin {
  background: #f0f7ff;
  border-left: 3px solid #2080f0;
}
body.dark .template-item.builtin {
  background: #1a2a3a;
  border-left-color: #3080d0;
}
.builtin-badge {
  display: inline-block;
  font-size: 10px;
  color: #2080f0;
  background: #e8f4ff;
  padding: 1px 6px;
  border-radius: 3px;
  margin-left: 8px;
  font-weight: 400;
}
body.dark .builtin-badge {
  color: #5ca0e0;
  background: #1a2a3a;
}

/* 拖拽区域 */
.drop-zone {
  position: relative;
  border: 2px dashed transparent;
  border-radius: 6px;
  transition: border-color 0.2s, background 0.2s;
}
.drop-zone.drag-over {
  border-color: #18a058;
  background: rgba(24, 160, 88, 0.06);
}
.drop-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  color: #18a058;
  font-weight: 600;
  background: rgba(24, 160, 88, 0.08);
  border-radius: 6px;
  pointer-events: none;
}
body.dark .drop-zone.drag-over { background: rgba(24, 160, 88, 0.1); }
body.dark .drop-overlay { color: #5fb67c; background: rgba(24, 160, 88, 0.1); }
</style>
