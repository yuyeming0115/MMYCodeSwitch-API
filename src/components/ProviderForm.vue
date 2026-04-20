<template>
  <div class="page">
    <header class="page-header">
      <n-button text size="large" @click="emit('back')">←</n-button>
      <span class="page-title">{{ isEdit ? t('edit') : t('add_provider') }}</span>
    </header>

    <div class="page-content">
      <n-form :model="form" label-placement="left" label-width="90px">
        <n-form-item :label="t('name')" required>
          <n-input v-model:value="form.name" />
        </n-form-item>
        <n-form-item :label="t('icon_fallback')">
          <n-input v-model:value="form.icon_fallback" maxlength="3" style="width:80px" />
          <n-button text style="margin-left:8px" @click="triggerIconUpload">{{ t('upload_icon') }}</n-button>
          <input ref="iconInput" type="file" accept=".png,.svg" style="display:none" @change="onIconFile" />
          <img v-if="iconPreview" :src="iconPreview" style="width:32px;height:32px;border-radius:6px;margin-left:8px;object-fit:cover" />
        </n-form-item>
        <n-form-item label="类型">
          <n-radio-group v-model:value="form.provider_type">
            <n-radio value="api">{{ t('type_api') }}</n-radio>
            <n-radio value="login">{{ t('type_login') }}</n-radio>
          </n-radio-group>
        </n-form-item>
        <template v-if="form.provider_type === 'api'">
          <n-form-item :label="t('api_key')">
            <n-input v-model:value="form.api_key_plain" type="password" show-password-on="click" :placeholder="t('api_key_placeholder')" />
          </n-form-item>
          <n-form-item :label="t('base_url')" required>
            <n-input v-model:value="form.base_url" placeholder="https://" />
          </n-form-item>
          <n-form-item :label="t('default_model')">
            <n-input v-model:value="form.models_default" placeholder="claude-opus-4-6" />
          </n-form-item>
        </template>
        <n-form-item :label="t('notes')">
          <n-input v-model:value="form.notes" type="textarea" :rows="2" />
        </n-form-item>
        <n-form-item label="">
          <n-button text @click="showPaste = !showPaste">{{ t('parse_paste') }}</n-button>
        </n-form-item>
        <template v-if="showPaste">
          <n-form-item label="">
            <n-input v-model:value="pasteText" type="textarea" :rows="4" :placeholder="t('paste_hint')" />
          </n-form-item>
          <n-form-item label="">
            <n-button @click="doParse">{{ t('parse') }}</n-button>
            <span v-if="parseResult" style="margin-left:8px;font-size:12px;color:#888">
              URL: {{ parseResult.baseUrl ?? '-' }} | Key: {{ parseResult.apiKey ? '***' : '-' }}
            </span>
            <n-button v-if="parseResult" text style="margin-left:8px" @click="applyParse">{{ t('apply') }}</n-button>
          </n-form-item>
        </template>
      </n-form>
    </div>

    <footer class="page-footer">
      <n-button size="large" @click="emit('back')">{{ t('cancel') }}</n-button>
      <n-button type="primary" size="large" :loading="saving" @click="submit">{{ t('save') }}</n-button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type Provider } from '../stores/app'
import { useMessage } from 'naive-ui'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()

const props = defineProps<{ provider?: Provider }>()
const emit = defineEmits<{ back: []; done: [] }>()

const isEdit = ref(false)
const saving = ref(false)
const showPaste = ref(false)
const pasteText = ref('')
const parseResult = ref<{ baseUrl?: string; apiKey?: string } | null>(null)
const iconInput = ref<HTMLInputElement | null>(null)
const iconPreview = ref('')
const pendingIconData = ref<{ base64: string; ext: string } | null>(null)

const form = ref({
  name: '',
  icon_fallback: '',
  provider_type: 'api',
  api_key_plain: '',
  base_url: '',
  models_default: '',
  notes: '',
})

watch(() => props.provider, (p) => {
  isEdit.value = !!p
  iconPreview.value = p?.icon_path ? resolveIconUrl(p.icon_path) : ''
  pendingIconData.value = null
  if (p) {
    form.value = { name: p.name, icon_fallback: p.icon_fallback, provider_type: p.provider_type, api_key_plain: '', base_url: p.base_url ?? '', models_default: p.models?.default ?? '', notes: p.notes ?? '' }
  } else {
    form.value = { name: '', icon_fallback: '', provider_type: 'api', api_key_plain: '', base_url: '', models_default: '', notes: '' }
  }
}, { immediate: true })

function triggerIconUpload() { iconInput.value?.click() }

function resolveIconUrl(iconPath: string): string {
  const normalized = iconPath.replace(/\\/g, '/')
  if (/^[A-Za-z]:\/|^\//.test(normalized)) {
    return `asset://localhost/${normalized}`
  }
  return `/${normalized}`
}

function onIconFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  const ext = file.name.endsWith('.svg') ? 'svg' : 'png'
  const reader = new FileReader()
  reader.onload = ev => {
    const dataUrl = ev.target?.result as string
    iconPreview.value = dataUrl
    pendingIconData.value = { base64: dataUrl.split(',')[1], ext }
  }
  reader.readAsDataURL(file)
}

async function doParse() {
  parseResult.value = await invoke('parse_paste', { text: pasteText.value })
}

function applyParse() {
  if (!parseResult.value) return
  if (parseResult.value.baseUrl) form.value.base_url = parseResult.value.baseUrl
  if (parseResult.value.apiKey) form.value.api_key_plain = parseResult.value.apiKey
  showPaste.value = false
}

async function submit() {
  if (!form.value.name) { msg.error(t('required')); return }
  saving.value = true
  try {
    const id = props.provider?.id ?? `provider_${Date.now()}`
    let icon_path: string | null = props.provider?.icon_path ?? null
    if (pendingIconData.value) {
      icon_path = await invoke<string>('save_provider_icon', { providerId: id, dataBase64: pendingIconData.value.base64, ext: pendingIconData.value.ext })
    }
    const input = {
      id: props.provider?.id ?? null,
      name: form.value.name.trim(),
      icon_fallback: form.value.icon_fallback || form.value.name.slice(0, 2),
      provider_type: form.value.provider_type,
      base_url: form.value.base_url || null,
      api_key_plain: form.value.api_key_plain || null,
      models: form.value.models_default ? { default: form.value.models_default } : null,
      notes: form.value.notes || null,
      icon_path,
    }
    await store.upsertProvider(input)
    msg.success(t('save_success'))
    emit('done')
  } catch (e) {
    msg.error('保存失败: ' + (e as Error)?.message ?? String(e))
  } finally {
    saving.value = false
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