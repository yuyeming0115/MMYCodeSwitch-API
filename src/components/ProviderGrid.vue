<template>
  <div class="grid">
    <div
      v-for="p in providers"
      :key="p.id"
      class="card"
      :class="{ active: p.id === activeProviderId }"
      @click="emit('switch', p)"
      @contextmenu.prevent="e => openMenu(e, p)"
    >
      <div class="icon">{{ p.icon_fallback }}</div>
      <div class="label">{{ p.name }}</div>
      <div v-if="p.id === activeProviderId" class="badge">✓</div>
    </div>
    <div class="card add-card" @click="emit('add')">
      <div class="icon">+</div>
      <div class="label">{{ t('add_provider') }}</div>
    </div>
  </div>

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
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Provider } from '../stores/app'

const { t } = useI18n()
defineProps<{ providers: Provider[]; activeProviderId?: string }>()
const emit = defineEmits<{ switch: [p: Provider]; edit: [p: Provider]; delete: [p: Provider]; add: [] }>()

const menuVisible = ref(false)
const menuX = ref(0)
const menuY = ref(0)
const menuTarget = ref<Provider | null>(null)

const menuOptions = [
  { label: () => t('edit'), key: 'edit' },
  { label: () => t('delete'), key: 'delete', props: { style: 'color:#d03050' } },
]

function openMenu(e: MouseEvent, p: Provider) {
  menuTarget.value = p
  menuX.value = e.clientX
  menuY.value = e.clientY
  menuVisible.value = true
}

function onMenuSelect(key: string) {
  menuVisible.value = false
  if (!menuTarget.value) return
  if (key === 'edit') emit('edit', menuTarget.value)
  else if (key === 'delete') emit('delete', menuTarget.value)
}
</script>

<style scoped>
.grid { display: flex; flex-wrap: wrap; gap: 12px; padding: 8px 0; }
.card {
  width: 96px; height: 96px; border-radius: 12px; border: 2px solid #e0e0e0;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  cursor: pointer; position: relative; transition: border-color .2s, box-shadow .2s;
  background: #fafafa;
}
.card:hover { border-color: #18a058; box-shadow: 0 2px 8px #18a05820; }
.card.active { border-color: #18a058; background: #f0faf5; }
.icon { font-size: 28px; font-weight: 700; color: #333; }
.label { font-size: 12px; margin-top: 4px; text-align: center; color: #555; max-width: 80px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.badge { position: absolute; top: 6px; right: 8px; color: #18a058; font-size: 14px; font-weight: 700; }
.add-card { border-style: dashed; background: #f5f5f5; }
.add-card .icon { color: #aaa; }
</style>
