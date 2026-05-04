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
        <div v-for="skill in store.skills" class="skill-item" :key="skill.name">
          <div class="skill-header">
            <span class="skill-name">{{ skill.name }}</span>
            <n-space>
              <n-button size="small" @click="editSkill(skill)">{{ t('edit') }}</n-button>
              <n-button size="small" type="error" @click="confirmDelete(skill.name)">{{ t('delete') }}</n-button>
            </n-space>
          </div>
          <pre class="skill-preview">{{ skill.content.slice(0, 100) }}...</pre>
        </div>
      </div>

      <n-button type="primary" @click="showAddModal = true" style="margin-top: 16px">
        {{ t('skill_add') }}
      </n-button>

      <!-- 添加/编辑模态框 -->
      <n-modal v-model:show="showAddModal" preset="dialog" :title="editing ? t('skill_edit') : t('skill_add')">
        <n-form-item :label="t('skill_name')">
          <n-input v-model:value="skillName" :disabled="editing" placeholder="simplify" />
        </n-form-item>
        <n-form-item :label="t('skill_content')">
          <n-input v-model:value="skillContent" type="textarea" :rows="10" placeholder="# skill name..." />
        </n-form-item>
        <template #action>
          <n-button @click="showAddModal = false">{{ t('cancel') }}</n-button>
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
  showAddModal.value = false
  skillName.value = ''
  skillContent.value = ''
  editing.value = false
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
.skill-list { display: flex; flex-direction: column; gap: 8px; }
.skill-item {
  padding: 12px;
  background: #f5f5f5;
  border-radius: 6px;
}
body.dark .skill-item { background: #2a2a2a; }
.skill-header { display: flex; align-items: center; justify-content: space-between; }
.skill-name { font-weight: 600; }
.skill-preview {
  font-size: 11px;
  color: #888;
  margin-top: 8px;
  white-space: pre-wrap;
  overflow: hidden;
}
</style>