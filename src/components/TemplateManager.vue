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
        <div v-for="tpl in store.templates" class="template-item" :key="tpl.name">
          <div class="template-header">
            <span class="template-name">{{ tpl.name }}</span>
            <n-space>
              <n-button size="small" @click="editTemplate(tpl)">{{ t('edit') }}</n-button>
              <n-button size="small" type="error" @click="confirmDelete(tpl.name)">{{ t('delete') }}</n-button>
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

      <!-- 添加/编辑模态框 -->
      <n-modal v-model:show="showAddModal" preset="dialog" :title="editing ? t('template_edit') : t('template_add')">
        <n-form-item :label="t('template_name')">
          <n-input v-model:value="templateName" :disabled="editing" placeholder="vue-standard" />
        </n-form-item>
        <n-form-item :label="t('template_content')">
          <n-input v-model:value="templateContent" type="textarea" :rows="10" placeholder="# 项目开发规范..." />
        </n-form-item>
        <template #action>
          <n-button @click="showAddModal = false">{{ t('cancel') }}</n-button>
          <n-button type="primary" @click="saveTemplate">{{ t('save') }}</n-button>
        </template>
      </n-modal>
    </div>

    <footer class="page-footer">
      <n-button size="large" @click="emit('back')">{{ t('cancel') }}</n-button>
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
  showAddModal.value = false
  templateName.value = ''
  templateContent.value = ''
  editing.value = false
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
}
.page-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
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
.empty-hint { color: #999; font-size: 13px; padding: 8px 0; }
.template-list, .binding-list { display: flex; flex-direction: column; gap: 8px; }
.template-item, .binding-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: #f5f5f5;
  border-radius: 6px;
}
body.dark .template-item, body.dark .binding-item { background: #2a2a2a; }
.template-header { display: flex; align-items: center; justify-content: space-between; width: 100%; }
.template-name { font-weight: 600; }
.binding-path { font-size: 12px; color: #666; max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
body.dark .binding-path { color: #888; }
.binding-template { font-size: 12px; color: #2080f0; margin-left: 8px; }
</style>