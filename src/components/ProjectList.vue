<template>
  <div class="project-list-section">
    <!-- 空状态 -->
    <div v-if="projects.length === 0" class="empty-state">
      <span class="empty-icon"></span>
      <p>{{ t('no_active_projects') }}</p>
    </div>

    <draggable
      v-else
      v-model="localProjects"
      item-key="id"
      :animation="200"
      :force-fallback="true"
      :fallback-tolerance="3"
      ghost-class="ghost"
      chosen-class="chosen"
      drag-class="dragging"
      class="project-cards"
      @start="onDragStart"
      @end="onDragEnd"
    >
      <template #item="{ element: proj }">
        <div class="proj-card">
          <!-- 左侧供应商图标（图片/首字母fallback） -->
          <div class="proj-icon" :class="{ 'no-img': !providerIconUrl(proj) }" :style="!providerIconUrl(proj) ? { background: providerIconBg(proj.provider_id) } : {}">
            <img v-if="providerIconUrl(proj)" class="proj-icon-img" :src="resolveIconUrl(providerIconUrl(proj)!)" :alt="proj.provider_name" />
            <span v-else class="proj-icon-text">{{ providerFirstLetter(proj) }}</span>
          </div>

          <!-- 中间信息区 -->
          <div class="proj-info">
            <div class="proj-name" :title="proj.project_path || ''">{{ proj.name || '未知项目' }}</div>
            <div class="proj-meta">
              <span class="proj-provider">{{ proj.provider_name || '未知供应商' }}</span>
              <span class="proj-time">{{ formatTime(proj.updated_at) }}</span>
            </div>
            <div class="proj-path" :title="proj.project_path || ''">{{ truncatePath(proj.project_path) }}</div>
          </div>

          <!-- 右侧按钮组 -->
          <div class="action-btns">
            <button
              class="launch-btn"
              :title="t('continue_dev')"
              @click.stop="handleLaunch(proj)"
            >▶</button>
            <button
              class="remove-btn"
              :title="t('remove_project')"
              @click.stop="handleRemove(proj)"
            >✕</button>
          </div>
        </div>
      </template>
    </draggable>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDialog, useMessage } from 'naive-ui'
import draggable from 'vuedraggable'
import type { ActiveProject, Provider } from '../stores/app'

const { t } = useI18n()
const msg = useMessage()
const dialog = useDialog()

const props = defineProps<{
  projects: ActiveProject[]
  providers?: Provider[]
}>()
const emit = defineEmits<{
  removed: [id: string]
  launch: [projectPath: string]
  reorder: [orderedIds: string[]]
}>()

// 拖拽状态
const isDragging = ref(false)
const localProjects = ref<ActiveProject[]>([])

// 同步 props → local（拖拽中不同步，避免重置拖拽状态）
watch(() => props.projects, (newVal) => {
  if (!isDragging.value) {
    localProjects.value = [...newVal]
  }
}, { immediate: true, deep: true })

function onDragStart() {
  isDragging.value = true
}

function onDragEnd() {
  isDragging.value = false
  const orderedIds = localProjects.value.map(p => p.id)
  emit('reorder', orderedIds)
}

/// 获取供应商的 icon_path（有则返回路径，无则返回 null）
function providerIconUrl(proj: ActiveProject): string | null {
  const p = (props.providers ?? []).find(x => x.id === proj.provider_id)
  return p?.icon_path ?? null
}

/// 将 icon_path 解析为完整 URL（与 ProviderGrid 保持一致）
function resolveIconUrl(iconPath: string): string {
  const normalized = iconPath.replace(/\\/g, '/')
  // 绝对路径 → Tauri asset 协议
  if (/^[A-Za-z]:\/|^\//.test(normalized)) {
    return `asset://localhost/${normalized}`
  }
  // 相对路径（内置图标）→ public 目录
  return `/${normalized}`
}

/// 没有图片时显示供应商名称首字母
function providerFirstLetter(proj: ActiveProject): string {
  const name = proj.provider_name || '?'
  return name.charAt(0).toUpperCase()
}

/// 根据provider_id生成稳定的渐变底色（hash取色）
const ICON_COLORS = [
  '#667eea', '#f093fb', '#4facfe', '#43e97b',
  '#fa709a', '#fee140', '#30cfd0', '#a8edea',
  '#ff9a9e', '#fecfef', '#ffecd2', '#fcb69f',
  '#a18cd1', '#fbc2eb', '#fad0c4', '#ffd1ff',
]
function providerIconBg(providerId: string): string {
  let hash = 0
  for (let i = 0; i < providerId.length; i++) {
    hash = ((hash << 5) - hash + providerId.charCodeAt(i)) | 0
  }
  return ICON_COLORS[Math.abs(hash) % ICON_COLORS.length]
}

function formatTime(iso: string): string {
  if (!iso) return ''
  const diff = Date.now() - new Date(iso).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return t('just_now')
  if (mins < 60) return `${mins}${t('mins_ago')}`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours}${t('hours_ago')}`
  const days = Math.floor(hours / 24)
  return `${days}${t('days_ago')}`
}

function truncatePath(path: string | undefined | null): string {
  if (!path) return ''
  if (path.length <= 50) return path
  return '...' + path.slice(-47)
}

function handleRemove(proj: ActiveProject) {
  dialog.warning({
    title: t('confirm_remove_title'),
    content: `${t('confirm_remove_msg')}「${proj.name || '未知项目'}」?`,
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: () => {
      emit('removed', proj.id)
      msg.success(`${t('project_removed')}`)
    },
  })
}

function handleLaunch(proj: ActiveProject) {
  if (proj.project_path) {
    emit('launch', proj.project_path)
  }
}
</script>

<style scoped>
.project-list-section {
  margin-top: 0;
  padding-top: 0;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.project-cards {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-content: flex-start;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 4px 0;
}

.empty-state {
  text-align: center;
  padding: 16px 0;
  color: #bbb;
}
.empty-state p {
  margin: 4px 0 0;
  font-size: 12px;
}
.empty-icon { font-size: 24px; }

/* ====== 卡片主体 ====== */
.proj-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1.5px solid #e8e8e8;
  background: #fff;
  transition: border-color .2s, box-shadow .2s;
  gap: 8px;
  cursor: grab;
  min-width: 200px;
  max-width: 320px;
  flex: 1 1 auto;
}
.proj-card:hover {
  border-color: #c0c0c0;
  box-shadow: 0 2px 10px rgba(0,0,0,0.07);
}
body.dark .proj-card {
  background: #252525;
  border-color: #3a3a3a;
}
body.dark .proj-card:hover { border-color: #555; }

/* ====== 左侧供应商图标（图片/首字母） ====== */
.proj-icon {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  box-shadow: 0 1px 3px rgba(0,0,0,0.12);
}
/* 有图标图片时 */
.proj-icon-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 50%;
}
/* 无图标时（显示首字母 + 渐变底色） */
.proj-icon.no-img {
  font-size: 14px;
  font-weight: 700;
  color: #fff;
  line-height: 1;
}
.proj-icon-text {
  user-select: none;
}

/* ====== 信息区 ====== */
.proj-info { flex: 1; min-width: 0; }
.proj-name {
  font-size: 13px;
  font-weight: 600;
  color: #333;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
body.dark .proj-name { color: #ddd; }

.proj-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
  font-size: 11px;
}
.proj-provider {
  color: #d77757;
  font-weight: 500;
}
.proj-time { color: #bbb; }

/* 隐藏路径显示，通过 tooltip 查看 */
.proj-path {
  display: none;
}

/* ====== 右侧按钮组 ====== */
.action-btns {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  opacity: 0;
  transition: opacity .15s;
}
.proj-card:hover .action-btns { opacity: 1; }

.launch-btn {
  width: 24px; height: 24px;
  border-radius: 6px;
  border: none;
  background: #d77757;
  color: #fff;
  cursor: pointer;
  font-size: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background .15s, transform .1s;
}
.launch-btn:hover {
  background: #c46a4a;
  transform: scale(1.08);
}
.launch-btn:active { transform: scale(0.95); }

.remove-btn {
  width: 20px; height: 20px;
  border-radius: 5px;
  border: none;
  background: transparent;
  color: #ccc;
  cursor: pointer;
  font-size: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all .15s;
}
.remove-btn:hover {
  background: #fee2e2;
  color: #d03050;
}
body.dark .remove-btn:hover { background: #3a1515; }

/* 移动端/小屏：按钮常驻显示 */
@media (max-width: 500px) {
  .action-btns { opacity: 1 !important; }
}

/* 拖拽视觉反馈（与 ProviderGrid 一致） */
.ghost {
  opacity: 0.2;
  border: 2px dashed #4A90D9;
  border-radius: 10px;
  min-width: 200px;
  max-width: 320px;
  flex: 1 1 auto;
}
.chosen {
  box-shadow: 0 4px 16px rgba(24,160,88,0.2);
}
.dragging {
  transform: scale(1.05) rotate(1deg);
  box-shadow: 0 16px 40px rgba(74, 144, 217, 0.3);
  z-index: 9999;
}
</style>

<!-- 深色模式滚动条（全局样式，不受 scoped 影响） -->
<style>
body.dark .project-cards { scrollbar-color: rgba(200,200,200,0.35) #2a2a2a !important; }
body.dark .project-cards::-webkit-scrollbar { background: #2a2a2a; }
body.dark .project-cards::-webkit-scrollbar-track { background: #2a2a2a; }
body.dark .project-cards::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.35) !important; }
body.dark .project-cards::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.5) !important; }
</style>
