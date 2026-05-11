<template>
  <div class="page">
    <header class="page-header">
      <n-button text size="large" @click="emit('back')">←</n-button>
      <span class="page-title">{{ t('skills') }}</span>
    </header>

    <div class="page-content">
      <p class="hint-text">{{ t('skill_hint') }}</p>

      <!-- Skill 列表 -->
      <n-divider>{{ t('skills') }}</n-divider>
      <div v-if="store.skills.length === 0" class="empty-hint">{{ t('none') }}</div>
      <div v-else class="skill-list">
        <div
          v-for="skill in store.skills"
          class="skill-item"
          :key="skill.name"
          @click="editSkill(skill)"
        >
          <div class="skill-header">
            <span class="skill-name">{{ skill.name }}</span>
            <n-button size="small" type="error" @click.stop="confirmDelete(skill.name)">{{ t('delete') }}</n-button>
          </div>
          <pre class="skill-preview">{{ skill.content.slice(0, 100) }}...</pre>
        </div>
      </div>

      <n-button type="primary" @click="showAddModal = true" style="margin-top: 16px">
        {{ t('skill_add') }}
      </n-button>

      <!-- 添加/编辑模态框 — 单文本框一键导入 -->
      <n-modal v-model:show="showAddModal" preset="dialog" :title="editing ? t('skill_edit') : t('skill_add')">
        <!-- 名称（只读，自动从内容第一行提取） -->
        <n-form-item :label="t('skill_name')">
          <n-input v-model:value="skillName" :disabled="true" :placeholder="t('skill_name_auto')" />
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
            v-model:value="skillContent"
            type="textarea"
            :rows="12"
            :placeholder="t('skill_content_placeholder')"
            @input="autoExtractName"
          />
          <div v-if="isDragOver" class="drop-overlay">{{ t('drop_file_hint') }}</div>
        </div>
        <template #action>
          <n-button @click="closeModal">{{ t('cancel') }}</n-button>
          <n-button type="primary" @click="saveSkill">{{ t('save') }}</n-button>
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
import { useAppStore, type Skill } from '../stores/app'
import { useMessage, useDialog } from 'naive-ui'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()
const emit = defineEmits<{ back: [] }>()

const showAddModal = ref(false)
const skillName = ref('')
const skillContent = ref('')
const editing = ref(false)
const isDragOver = ref(false)

/** 从内容第一行自动提取名称（markdown # 标题） */
function autoExtractName() {
  const firstLine = skillContent.value.split('\n')[0]?.trim()
  const match = firstLine?.match(/^#\s+(.+)/)
  if (match && match[1]) {
    skillName.value = match[1].trim()
  }
}

function editSkill(skill: Skill) {
  editing.value = true
  skillName.value = skill.name
  skillContent.value = skill.content
  showAddModal.value = true
}

async function saveSkill() {
  if (!skillName.value.trim()) {
    msg.error(t('required'))
    return
  }
  await store.saveSkill(skillName.value.trim(), skillContent.value)
  msg.success(t('skill_save_success'))
  closeModal()
}

function closeModal() {
  showAddModal.value = false
  skillName.value = ''
  skillContent.value = ''
  editing.value = false
  isDragOver.value = false
}

/** 拖入 .md 文件：读取文件名作为名称，内容作为 Skill 内容 */
async function onFileDrop(e: DragEvent) {
  isDragOver.value = false
  const file = e.dataTransfer?.files?.[0]
  if (!file) return
  if (!file.name.endsWith('.md') && !file.name.endsWith('.txt')) {
    msg.error(t('skill_drop_invalid_file'))
    return
  }
  // 用 Tauri fs 读取文件内容（通过 path）
  // 浏览器端无法直接获取 path，改用 FileReader 读取
  const reader = new FileReader()
  reader.onload = () => {
    skillContent.value = reader.result as string
    // 用文件名（不含扩展名）作为 Skill 名
    skillName.value = file.name.replace(/\.(md|txt)$/, '')
    autoExtractName()
  }
  reader.readAsText(file)
}

function confirmDelete(name: string) {
  dialog.warning({
    title: t('confirm'),
    content: t('skill_delete_confirm', { name }),
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: async () => {
      await store.deleteSkill(name)
      msg.success(t('skill_delete_success'))
    }
  })
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
.skill-list { display: flex; flex-direction: column; gap: 8px; }
.skill-item {
  padding: 12px;
  background: #f5f5f5;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
  border: 1px solid transparent;
}
.skill-item:hover {
  background: #f0f0f0;
  border-color: #18a058;
}
body.dark .skill-item { background: #2a2a2a; }
body.dark .skill-item:hover {
  background: #333;
  border-color: #18a058;
}
.skill-header { display: flex; align-items: center; justify-content: space-between; }
.skill-name { font-weight: 600; }
.skill-preview {
  font-size: 11px;
  color: #888;
  margin-top: 8px;
  white-space: pre-wrap;
  overflow: hidden;
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
