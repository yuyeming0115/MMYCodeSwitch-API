<template>
  <draggable
    v-model="localProviders"
    item-key="id"
    :animation="200"
    :force-fallback="true"
    :fallback-tolerance="3"
    ghost-class="ghost"
    chosen-class="chosen"
    drag-class="dragging"
    class="grid"
    :class="{ compact }"
    @start="onDragStart"
    @end="onDragEnd"
  >
    <template #item="{ element: p }">
      <div
        class="card"
        :class="{ active: p.id === activeProviderId }"
        @click="emit('switch', p)"
        @contextmenu.prevent="e => openMenu(e, p)"
      >
        <img v-if="p.icon_path" :src="resolveIconUrl(p.icon_path)" class="icon-img" />
        <div v-else class="icon-wrap"><span class="icon">{{ p.icon_fallback || p.name?.charAt(0) || '?' }}</span></div>
        <div class="label">{{ p.name }}</div>
        <div v-if="p.id === activeProviderId" class="badge">✓</div>
        <div v-if="testState[p.id]" class="test-badge" :class="testState[p.id]">
          {{ testState[p.id] === 'testing' ? '…' : testState[p.id] === 'ok' ? '✓' : '✗' }}
        </div>
      </div>
    </template>
  </draggable>

  <n-dropdown
    trigger="manual"
    :x="menuX"
    :y="menuY"
    :show="menuVisible"
    :options="menuOptions"
    @clickoutside="menuVisible = false"
    @select="onMenuSelect"
  />
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import draggable from 'vuedraggable'
import type { Provider } from '../stores/app'

const { t } = useI18n()
const msg = useMessage()
const props = defineProps<{ providers: Provider[]; activeProviderId?: string; compact?: boolean }>()
const emit = defineEmits<{ switch: [p: Provider]; edit: [p: Provider]; delete: [p: Provider]; add: []; reorder: [orderedIds: string[]] }>()

const menuVisible = ref(false)
const menuX = ref(0)
const menuY = ref(0)
const menuTarget = ref<Provider | null>(null)
const testState = ref<Record<string, 'testing' | 'ok' | 'fail'>>({})

// 拖拽状态
const isDragging = ref(false)

// 本地可变列表，用于 vuedraggable v-model
const localProviders = ref<Provider[]>([])

// 同步 props → local（拖拽中不同步，避免重置拖拽状态）
watch(() => props.providers, (newVal) => {
  if (!isDragging.value) {
    localProviders.value = [...newVal]
  }
}, { immediate: true, deep: true })

const menuOptions = [
  { label: () => t('edit'), key: 'edit' },
  { label: () => t('test_connection'), key: 'test' },
  { label: () => t('delete'), key: 'delete', props: { style: 'color:#d03050' } },
]

/** 解析图标 URL：支持绝对路径（用户上传）和相对路径（内置/public 图标） */
function resolveIconUrl(iconPath: string): string {
  const normalized = iconPath.replace(/\\/g, '/')
  // 绝对路径（如 C:\Users\... 或 /home/...）→ 用 Tauri asset 协议读取用户数据目录的文件
  if (/^[A-Za-z]:\/|^\//.test(normalized)) {
    return `asset://localhost/${normalized}`
  }
  // 相对路径（如 icons/dashscope.svg）→ 指向 public/ 目录，Vite 直接服务
  return `/${normalized}`
}

function openMenu(e: MouseEvent, p: Provider) {
  menuTarget.value = p
  menuX.value = e.clientX
  menuY.value = e.clientY
  menuVisible.value = true
}

async function onMenuSelect(key: string) {
  menuVisible.value = false
  if (!menuTarget.value) return
  if (key === 'edit') emit('edit', menuTarget.value)
  else if (key === 'delete') emit('delete', menuTarget.value)
  else if (key === 'test') {
    const id = menuTarget.value.id
    const name = menuTarget.value.name
    testState.value[id] = 'testing'
    msg.loading(`正在测试「${name}」连通性...`, { duration: 0 })
    try {
      const ok = await invoke<boolean>('test_provider', { providerId: id })
      testState.value[id] = ok ? 'ok' : 'fail'
      if (ok) {
        msg.success(`✅ 「${name}」连通正常`)
      } else {
        msg.warning(`⚠️ 「${name}」响应异常（可能 Key 无效或网络不通）`)
      }
    } catch (e) {
      testState.value[id] = 'fail'
      msg.error(`❌ 「${name}」测试失败: ${e}`)
    } finally {
      // 关闭 loading toast
      msg.destroyAll()
    }
    setTimeout(() => { delete testState.value[id] }, 4000)
  }
}

// ── 拖拽排序 ──

function onDragStart() {
  isDragging.value = true
}

function onDragEnd() {
  isDragging.value = false
  const orderedIds = localProviders.value.map(p => p.id)
  emit('reorder', orderedIds)
}
</script>

<style scoped>
.grid {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  padding: 8px 0;
  align-content: center;
  justify-content: center;
  user-select: none;
}
.grid.compact {
  height: 100%;
  align-content: center;
}
.card {
  width: 108px; min-height: 100px; border-radius: 14px; border: 2px solid #e0e0e0;
  display: flex; flex-direction: column; align-items: center; justify-content: flex-start;
  padding: 14px 8px 10px;
  cursor: grab; position: relative;
  /* 只对非 transform 属性设置 transition，避免覆盖 SortableJS 内联动画 */
  transition: border-color .2s, box-shadow .2s;
  background: #fff; user-select: none;
}
.card:hover { border-color: #18a058; box-shadow: 0 3px 10px rgba(24,160,88,0.16); }
.card.active { border-color: #18a058; background: #f0faf5; }

/* 深色模式适配 */
body.dark .card { background: #2a2a2a; border-color: #444; }
body.dark .card:hover { border-color: #18a058; }
body.dark .card.active { background: #1a3a28; border-color: #18a058; }
body.dark .label { color: #ccc; }

/* 拖拽视觉反馈 */
.ghost {
  opacity: 0.2;
  border: 2px dashed #4A90D9;
  border-radius: 14px;
}
.chosen {
  box-shadow: 0 4px 16px rgba(24,160,88,0.2);
}
.dragging {
  transform: scale(1.05) rotate(1deg);
  box-shadow: 0 16px 40px rgba(74, 144, 217, 0.3);
  z-index: 9999;
}

/* 图标区域：彩色圆角背景（与 QuickSetup 风格统一） */
.icon-wrap {
  width: 48px; height: 48px;
  border-radius: 13px;
  border: 1.5px solid #e0e0e0;
  display: flex; align-items: center; justify-content: center;
  margin-bottom: 8px;
  background: linear-gradient(135deg, #f8f8f8, #eee);
  transition: border-color 0.2s, box-shadow 0.2s;
}
body.dark .icon-wrap {
  background: linear-gradient(135deg, #333, #3a3a3a);
  border-color: #555;
}
.card:hover .icon-wrap,
.card.active .icon-wrap {
  border-color: #18a05860;
  box-shadow: 0 2px 6px rgba(24,160,88,0.15);
}
.icon {
  font-size: 18px;
  line-height: 1;
  color: #555;
  font-weight: 700;
}
body.dark .icon { color: #ccc; }

.label {
  font-size: 12px; font-weight: 600;
  text-align: center; color: #444;
  max-width: 90px;
  line-height: 1.35;
  /* 允许换行，最多2行 */
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}

.badge { position: absolute; top: 5px; right: 7px; color: #18a058; font-size: 14px; font-weight: 700; }
.test-badge { position: absolute; bottom: 5px; right: 7px; font-size: 11px; font-weight: 700; }
.test-badge.testing { color: #aaa; }
.test-badge.ok { color: #18a058; }
.test-badge.fail { color: #d03050; }
.icon-img { width: 40px; height: 40px; border-radius: 8px; object-fit: cover; }
</style>
