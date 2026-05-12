<template>
  <div class="page">
    <div class="page-content">
      <!-- 顶部：模板选择区（仅新建时显示） -->
      <div v-if="!isEdit" class="template-section">
        <div class="template-header">
          <span class="section-title">{{ t('select_template') }}</span>
          <n-button text size="small" type="primary" @click="showAddTemplateModal = true">
            + {{ t('add_custom_template') }}
          </n-button>
        </div>

        <!-- 内置模板 -->
        <div class="template-grid">
          <div
            v-for="tpl in builtinTemplates"
            :key="tpl.id"
            class="tpl-card"
            :class="{ selected: selectedTemplateId === tpl.id }"
            @click="selectTemplate(tpl)"
          >
          <div class="tpl-icon-wrap">
              <img v-if="tpl.builtinIcon" :src="`/${tpl.builtinIcon}`" class="tpl-icon-img" />
              <span v-else class="tpl-icon">{{ tpl.icon }}</span>
            </div>
            <div class="tpl-name">{{ tpl.name }}</div>
            <div class="tpl-desc">{{ tpl.description }}</div>
            <div v-if="tpl.badge" class="tpl-badge" :style="{ background: tpl.color }">{{ tpl.badge }}</div>
          </div>
        </div>

        <!-- 用户自定义模板 -->
        <div v-if="customTemplates.length > 0" class="template-divider">
          <span>{{ t('custom_templates') }}</span>
        </div>
        <div v-if="customTemplates.length > 0" class="template-grid">
          <div
            v-for="tpl in customTemplates"
            :key="tpl.id"
            class="tpl-card custom"
            :class="{ selected: selectedTemplateId === tpl.id }"
            @click="selectTemplate(tpl)"
          >
            <div class="tpl-icon-wrap" style="background: #f5f5f5; borderColor: #ddd">
              <span class="tpl-icon">{{ tpl.icon || '📦' }}</span>
            </div>
            <div class="tpl-name">{{ tpl.name }}</div>
            <div class="tpl-desc">{{ tpl.description || t('custom_template') }}</div>
            <n-button
              size="tiny"
              text
              type="error"
              class="tpl-delete"
              @click.stop="confirmDeleteTemplate(tpl.id)"
            >×</n-button>
          </div>
        </div>

        <n-divider />
      </div>

      <!-- 配置表单 -->
      <n-form :model="form" label-placement="left" label-width="90px">
        <!-- 编辑模式下"保存为模板"快捷入口 -->
        <n-form-item v-if="isEdit" label="">
          <n-button text type="warning" size="small" @click="showSaveAsTemplateModal = true">
            {{ t('save_as_template') }}
          </n-button>
        </n-form-item>
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
            <n-input v-model:value="form.api_key_plain" type="password" show-password-on="click" :placeholder="keyPlaceholder || t('api_key_placeholder')" />
          </n-form-item>

          <!-- 多 Base URL 选择（模板有多个选项时显示） -->
          <n-form-item v-if="currentTemplate && currentTemplate.baseUrls.length > 1" label="接口协议">
            <n-radio-group v-model:value="selectedBaseUrlIndex">
              <n-space>
                <n-radio v-for="(url, idx) in currentTemplate.baseUrls" :key="idx" :value="idx">
                  <span>{{ url.label }}</span>
                  <span v-if="url.hint" style="font-size:11px;color:#999;margin-left:4px">({{ url.hint }})</span>
                </n-radio>
              </n-space>
            </n-radio-group>
            <div v-if="protocolHint" style="font-size:11px;color:#e89834;margin-top:4px">{{ protocolHint }}</div>
          </n-form-item>

          <!-- Base URL 输入框（始终显示，让用户检查确认） -->
          <n-form-item :label="t('base_url')" required>
            <n-input v-model:value="form.base_url" placeholder="https://" />
          </n-form-item>

          <!-- 默认模型 + 刷新按钮 -->
          <n-form-item :label="t('default_model')">
            <div style="display:flex;align-items:center;gap:8px;width:100%">
              <n-select
                v-model:value="form.models_default"
                :options="modelOptions"
                :placeholder="t('select_default_model')"
                filterable
                style="flex:1"
              />
              <!-- 刷新按钮（选择模板后显示） -->
              <n-button
                v-if="currentTemplate && form.api_key_plain"
                type="primary"
                size="small"
                :loading="refreshingModels"
                @click="refreshModels"
              >
                {{ refreshingModels ? t('refreshing_models') : t('refresh_models') }}
              </n-button>
            </div>
            <!-- 刷新提示 -->
            <div v-if="refreshedModels.length > 0" style="font-size:11px;color:#18a058;margin-top:4px">
              {{ t('models_refreshed', { n: refreshedModels.length }) }}
            </div>
            <div v-else-if="currentTemplate && !form.api_key_plain" style="font-size:11px;color:#999;margin-top:4px">
              {{ t('refresh_hint') }}
            </div>
          </n-form-item>

          <!-- 模型列表 -->
          <n-form-item label="模型列表">
            <div style="width:100%">
              <n-input
                v-model:value="customModelInput"
                type="textarea"
                :rows="3"
                :placeholder="t('model_list_placeholder')"
              />
              <div style="display:flex;justify-content:space-between;margin-top:4px">
                <span v-if="parsedModelCount > 0" style="font-size:11px;color:#18a058">
                  {{ t('parsed_models', { n: parsedModelCount }) }}
                </span>
                <span v-else style="font-size:11px;color:#999">
                  {{ t('model_list_hint') }}
                </span>
                <n-button v-if="customModelInput" size="tiny" text type="error" @click="clearCustomModels">{{ t('clear') }}</n-button>
              </div>
            </div>
          </n-form-item>

          <!-- 模板帮助链接 -->
          <n-alert v-if="currentTemplate && currentTemplate.helpUrl" type="info" :bordered="false" style="margin-bottom:16px">
            {{ t('config_help') }}：
            <n-button text type="primary" @click="openHelp(currentTemplate.helpUrl!)">{{ currentTemplate.helpUrl }}</n-button>
          </n-alert>
        </template>
        <n-form-item :label="t('notes')">
          <n-input v-model:value="form.notes" type="textarea" :rows="2" />
        </n-form-item>

        <!-- 从配置解析（折叠） -->
        <n-form-item label="">
          <n-button text @click="showPaste = !showPaste">{{ t('parse_paste') }}</n-button>
        </n-form-item>
        <template v-if="showPaste">
          <n-form-item label="">
            <n-input v-model:value="pasteText" type="textarea" :rows="6" :placeholder="t('paste_hint')" />
          </n-form-item>
          <n-form-item label="">
            <n-button @click="doParse">{{ t('parse') }}</n-button>
            <span v-if="parseResult" style="margin-left:8px;font-size:12px;color:#888">
              URL: {{ parseResult.baseUrl ?? '-' }} | Key: {{ parseResult.apiKey ? '***' : '-' }}
              <template v-if="parseResult.models && parseResult.models.length"> | {{ t('models_count', { n: parseResult.models.length }) }}</template>
              <template v-if="parseResult.source"> | <span style="color:#18a058">{{ parseResult.source }}</span></template>
            </span>
            <n-button v-if="parseResult" text style="margin-left:8px" @click="applyParse">{{ t('apply') }}</n-button>
          </n-form-item>
        </template>
      </n-form>
    </div>

    <!-- 添加自定义模板对话框 -->
    <n-modal v-model:show="showAddTemplateModal" preset="dialog" :title="t('add_custom_template')" style="width:600px">
      <n-tabs v-model:value="addTemplateTab">
        <n-tab-pane name="paste" :tab="t('paste_parse')">
          <n-input
            v-model:value="newTemplatePasteText"
            type="textarea"
            :rows="6"
            :placeholder="t('paste_template_hint')"
          />
          <n-button type="primary" style="margin-top:8px" @click="parseTemplateFromPaste">
            {{ t('parse') }}
          </n-button>
          <div v-if="parsedTemplatePreview" style="margin-top:12px;padding:12px;background:#f5f5f5;border-radius:8px">
            <div style="font-weight:600;margin-bottom:8px">{{ t('parse_result') }}</div>
            <div v-for="(v, k) in parsedTemplatePreview" :key="k" style="font-size:12px">
              <span style="color:#666">{{ k }}:</span> {{ v }}
            </div>
          </div>
        </n-tab-pane>
        <n-tab-pane name="manual" :tab="t('manual_input')">
          <n-form label-placement="left" label-width="80px">
            <n-form-item :label="t('template_name')" required>
              <n-input v-model:value="newTemplateName" />
            </n-form-item>
            <n-form-item :label="t('base_url')" required>
              <n-input v-model:value="newTemplateBaseUrl" placeholder="https://" />
            </n-form-item>
            <n-form-item :label="t('icon')">
              <n-input v-model:value="newTemplateIcon" maxlength="2" style="width:60px" />
            </n-form-item>
            <n-form-item :label="t('models')">
              <n-input v-model:value="newTemplateModels" type="textarea" :rows="2" :placeholder="t('models_placeholder')" />
            </n-form-item>
            <n-form-item :label="t('description')">
              <n-input v-model:value="newTemplateDesc" />
            </n-form-item>
          </n-form>
        </n-tab-pane>
      </n-tabs>
      <template #action>
        <n-button @click="showAddTemplateModal = false">{{ t('cancel') }}</n-button>
        <n-button type="primary" @click="saveNewTemplate" :disabled="!canSaveNewTemplate">
          {{ t('save') }}
        </n-button>
      </template>
    </n-modal>

    <!-- 从供应商保存为模板对话框 -->
    <n-modal v-model:show="showSaveAsTemplateModal" preset="dialog" :title="t('save_as_template')" style="width:500px">
      <p style="font-size:12px;color:#888;margin-bottom:12px">{{ t('save_as_template_hint') }}</p>
      <n-form label-placement="left" label-width="80px">
        <n-form-item :label="t('template_name')" required>
          <n-input v-model:value="saveAsTemplateName" :placeholder="form.name" />
        </n-form-item>
        <n-form-item :label="t('base_url')">
          <n-input :value="form.base_url" disabled />
        </n-form-item>
        <n-form-item :label="t('models')">
          <div style="font-size:12px;color:#666">
            {{ refreshedModels.length > 0 ? refreshedModels.join(', ') : (parsedModels.length > 0 ? parsedModels.join(', ') : form.models_default || '-') }}
          </div>
        </n-form-item>
        <n-form-item :label="t('description')">
          <n-input v-model:value="saveAsTemplateDesc" :placeholder="t('save_as_template_desc_placeholder', { name: form.name })" />
        </n-form-item>
      </n-form>
      <template #action>
        <n-button @click="showSaveAsTemplateModal = false">{{ t('cancel') }}</n-button>
        <n-button type="primary" @click="saveAsTemplate">{{ t('save') }}</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type Provider, type ProviderTemplate } from '../stores/app'
import { useMessage, useDialog } from 'naive-ui'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const dialog = useDialog()

const props = defineProps<{ provider?: Provider }>()

const isEdit = ref(false)
const showPaste = ref(false)
const pasteText = ref('')
const customModelInput = ref('')
const parseResult = ref<{ baseUrl?: string; apiKey?: string; models?: string[]; source?: string } | null>(null)
const iconInput = ref<HTMLInputElement | null>(null)
const iconPreview = ref('')
const pendingIconData = ref<{ base64: string; ext: string } | null>(null)

const selectedTemplateId = ref<string | null>(null)
const selectedBaseUrlIndex = ref(0)
/** 记录选中模板的内置图标路径（用于提交时自动关联 public/icons/*.svg） */
const selectedBuiltinIcon = ref<string | null>(null)

// 模型刷新状态
const refreshingModels = ref(false)
const refreshedModels = ref<string[]>([])
const refreshError = ref<string | null>(null)

// 添加自定义模板
const showAddTemplateModal = ref(false)
const addTemplateTab = ref<'paste' | 'manual'>('paste')
const newTemplatePasteText = ref('')
const parsedTemplatePreview = ref<Record<string, string> | null>(null)
const newTemplateName = ref('')
const newTemplateBaseUrl = ref('')
const newTemplateIcon = ref('')
const newTemplateModels = ref('')
const newTemplateDesc = ref('')

// 保存为模板（从现有供应商）
const showSaveAsTemplateModal = ref(false)
const saveAsTemplateName = ref('')
const saveAsTemplateDesc = ref('')

const form = ref({
  name: '',
  icon_fallback: '',
  provider_type: 'api',
  api_key_plain: '',
  base_url: '',
  models_default: '',
  notes: '',
})

// 模板分组
const builtinTemplates = computed(() => store.providerTemplates.filter(t => t.builtin))
const customTemplates = computed(() => store.providerTemplates.filter(t => !t.builtin))
const currentTemplate = computed(() => store.providerTemplates.find(t => t.id === selectedTemplateId.value))

const keyPlaceholder = computed(() => currentTemplate.value?.keyPlaceholder || '')

const protocolHint = computed(() => {
  const tpl = currentTemplate.value
  if (!tpl || tpl.baseUrls.length <= 1) return ''
  return tpl.baseUrls[selectedBaseUrlIndex.value]?.protocolHint ?? ''
})

const parsedModels = computed(() => {
  if (!customModelInput.value.trim()) return []
  return customModelInput.value
    .split(/[\n,，\s]+/)
    .map(s => s.trim())
    .filter(Boolean)
    .filter((v, i, arr) => arr.indexOf(v) === i)
})

const parsedModelCount = computed(() => parsedModels.value.length)

const modelOptions = computed(() => {
  const tpl = currentTemplate.value
  // 优先使用刷新后的模型列表
  const source = refreshedModels.value.length > 0
    ? refreshedModels.value
    : (parsedModels.value.length > 0
      ? parsedModels.value
      : (tpl?.models ?? []))
  const additional = form.value.models_default ? [form.value.models_default] : []
  const all = [...new Set([...source, ...additional])]
  return all.map(m => ({ label: m, value: m }))
})

// 能否保存新模板
const canSaveNewTemplate = computed(() => {
  if (addTemplateTab.value === 'paste') {
    return parsedTemplatePreview.value && newTemplateName.value.trim()
  } else {
    return newTemplateName.value.trim() && newTemplateBaseUrl.value.trim()
  }
})

onMounted(async () => {
  await store.loadProviderTemplates()
})

watch(() => props.provider, (p) => {
  isEdit.value = !!p
  iconPreview.value = p?.icon_path ? resolveIconUrl(p.icon_path) : ''
  pendingIconData.value = null
  if (p) {
    let pt = p.provider_type
    if (pt !== 'login' && pt !== 'api') {
      pt = (p.base_url) ? 'api' : 'login'
    }
    form.value = {
      name: p.name,
      icon_fallback: p.icon_fallback || p.name.slice(0, 3),
      provider_type: pt,
      api_key_plain: '',
      base_url: p.base_url ?? '',
      models_default: p.models?.default ?? '',
      notes: p.notes ?? '',
    }
    customModelInput.value = ''
    selectedTemplateId.value = null
  } else {
    resetForm()
  }
}, { immediate: true })

// ★ 监听协议选择变化，同步更新 base_url
watch(selectedBaseUrlIndex, (idx) => {
  const tpl = currentTemplate.value
  if (tpl && tpl.baseUrls.length > 1) {
    form.value.base_url = tpl.baseUrls[idx]?.value || ''
  }
})

function resetForm() {
  form.value = { name: '', icon_fallback: '', provider_type: 'api', api_key_plain: '', base_url: '', models_default: '', notes: '' }
  customModelInput.value = ''
  selectedTemplateId.value = null
  selectedBuiltinIcon.value = null
  selectedBaseUrlIndex.value = 0
}

function selectTemplate(tpl: ProviderTemplate) {
  selectedTemplateId.value = tpl.id
  selectedBuiltinIcon.value = tpl.builtinIcon || null  // ★ 记录内置图标路径
  form.value.name = tpl.name
  form.value.icon_fallback = tpl.iconFallback || tpl.name.slice(0, 3)
  form.value.base_url = tpl.baseUrls[0]?.value || ''
  if (tpl.models.length > 0) {
    form.value.models_default = tpl.models[0]
  }
  form.value.notes = t('template_from', { name: tpl.name })
  selectedBaseUrlIndex.value = 0

  // ★ 清空刷新状态
  refreshedModels.value = []
  refreshError.value = null

  // ★ 预览内置图标（让用户能看到选择的模板图标）
  if (tpl.builtinIcon) {
    iconPreview.value = `/${tpl.builtinIcon}`
  }
}

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
  const raw = await invoke<{ baseUrl?: string; apiKey?: string; models?: string[] }>('parse_paste', { text: pasteText.value })
  let source = ''
  if (raw.baseUrl && raw.apiKey) {
    source = '✅ 智能识别成功'
  } else if (raw.baseUrl || raw.apiKey) {
    source = '⚠️ 识别部分字段'
  }
  if (raw.models && raw.models.length > 0) {
    source += ` · 提取 ${raw.models.length} 个模型`
  }
  parseResult.value = raw ? { ...raw, source } : null
}

function applyParse() {
  if (!parseResult.value) return
  if (parseResult.value.baseUrl) form.value.base_url = parseResult.value.baseUrl
  if (parseResult.value.apiKey) form.value.api_key_plain = parseResult.value.apiKey
  if (parseResult.value.models && parseResult.value.models.length > 0) {
    customModelInput.value = parseResult.value.models.join('\n')
  }
  showPaste.value = false
}

function clearCustomModels() {
  customModelInput.value = ''
}

function openHelp(url: string) {
  invoke('tauri', { __tauriModule: 'shell', message: { cmd: 'open', payload: url } })
}

// ★ 刷新模型列表（调用供应商 API 实时获取）
async function refreshModels() {
  const tpl = currentTemplate.value
  if (!tpl || !form.value.api_key_plain || !form.value.base_url) {
    msg.warning(t('refresh_hint'))
    return
  }

  refreshingModels.value = true
  refreshError.value = null

  try {
    const result = await invoke<{ models: string[]; cached_at: string }>('refresh_template_models', {
      templateId: tpl.id,
      baseUrl: form.value.base_url,
      apiKey: form.value.api_key_plain,
    })
    refreshedModels.value = result.models
    // 自动选择第一个模型作为默认
    if (result.models.length > 0 && !form.value.models_default) {
      form.value.models_default = result.models[0]
    }
    msg.success(t('models_refreshed', { n: result.models.length }))
  } catch (e) {
    refreshError.value = String(e)
    msg.error(t('refresh_failed', { msg: e }))
  } finally {
    refreshingModels.value = false
  }
}

async function parseTemplateFromPaste() {
  const raw = await invoke<{ baseUrl?: string; apiKey?: string; models?: string[] }>('parse_paste', { text: newTemplatePasteText.value })
  if (raw.baseUrl || raw.models?.length) {
    parsedTemplatePreview.value = {
      baseUrl: raw.baseUrl || '-',
      models: raw.models?.length ? raw.models.join(', ') : '-',
    }
    // 自动填充 URL
    if (raw.baseUrl) newTemplateBaseUrl.value = raw.baseUrl
    if (raw.models?.length) newTemplateModels.value = raw.models.join('\n')
    msg.success(t('parse_success'))
  } else {
    msg.warning(t('parse_failed'))
  }
}

async function saveNewTemplate() {
  if (!newTemplateName.value.trim()) {
    msg.error(t('template_name_required'))
    return
  }

  const models = newTemplateModels.value.trim()
    ? newTemplateModels.value.split(/[\n,，\s]+/).map(s => s.trim()).filter(Boolean)
    : []

  const input = {
    id: null,
    name: newTemplateName.value.trim(),
    icon: newTemplateIcon.value || '📦',
    color: '#666666',
    description: newTemplateDesc.value || null,
    builtinIcon: null,
    iconFallback: newTemplateName.value.slice(0, 3),
    baseUrls: [{ label: 'API', value: newTemplateBaseUrl.value }],
    models,
    keyPlaceholder: null,
    helpUrl: null,
    badge: null,
  }

  try {
    await store.saveProviderTemplate(input)
    msg.success(t('template_save_success'))
    showAddTemplateModal.value = false
    // 清空输入
    newTemplateName.value = ''
    newTemplateBaseUrl.value = ''
    newTemplateIcon.value = ''
    newTemplateModels.value = ''
    newTemplateDesc.value = ''
    newTemplatePasteText.value = ''
    parsedTemplatePreview.value = null
  } catch (e) {
    msg.error(t('save_failed') + ': ' + e)
  }
}

function confirmDeleteTemplate(id: string) {
  dialog.warning({
    title: t('confirm'),
    content: t('template_delete_confirm', { name: store.providerTemplates.find(t => t.id === id)?.name || '' }),
    positiveText: t('confirm'),
    negativeText: t('cancel'),
    onPositiveClick: async () => {
      try {
        await store.deleteProviderTemplate(id)
        msg.success(t('template_delete_success'))
        if (selectedTemplateId.value === id) {
          selectedTemplateId.value = null
        }
      } catch (e) {
        msg.error(String(e))
      }
    },
  })
}

// 从当前供应商保存为模板
async function saveAsTemplate() {
  if (!saveAsTemplateName.value.trim()) {
    msg.error(t('template_name_required'))
    return
  }
  if (!form.value.base_url) {
    msg.error(t('base_url_required'))
    return
  }

  // 收集模型列表
  const models = [
    ...refreshedModels.value,
    ...parsedModels.value,
    form.value.models_default
  ].filter(Boolean) as string[]
  const uniqueModels = [...new Set(models)]

  const input = {
    id: null,
    name: saveAsTemplateName.value.trim(),
    icon: form.value.icon_fallback.slice(0, 3) || form.value.name.slice(0, 3),
    color: '#666666',
    description: saveAsTemplateDesc.value || `来自供应商「${form.value.name}」`,
    builtinIcon: null,
    iconFallback: form.value.icon_fallback || form.value.name.slice(0, 3),
    baseUrls: [{ label: 'API', value: form.value.base_url }],
    models: uniqueModels.length > 0 ? uniqueModels : [form.value.models_default || ''],
    keyPlaceholder: null,
    helpUrl: null,
    badge: null,
  }

  try {
    await store.saveProviderTemplate(input)
    msg.success(t('template_save_success'))
    showSaveAsTemplateModal.value = false
    saveAsTemplateName.value = ''
    saveAsTemplateDesc.value = ''
    // 刷新模板列表
    await store.loadProviderTemplates()
  } catch (e) {
    msg.error(t('save_failed') + ': ' + e)
  }
}

// 防抖自动保存
let saveTimer: ReturnType<typeof setTimeout> | null = null
watch([form, pendingIconData], () => {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    if (!form.value.name?.trim()) return
    await autoSave()
  }, 800)
}, { deep: true })

async function autoSave() {
  try {
    const id = props.provider?.id ?? `provider_${Date.now()}`
    let icon_path: string | null = props.provider?.icon_path ?? null

    if (pendingIconData.value) {
      icon_path = await invoke<string>('save_provider_icon', {
        providerId: id, dataBase64: pendingIconData.value.base64, ext: pendingIconData.value.ext
      })
      pendingIconData.value = null
    }
    else if (!icon_path && selectedBuiltinIcon.value) {
      icon_path = selectedBuiltinIcon.value
    }

    await store.upsertProvider({
      id: props.provider?.id ?? null,
      name: form.value.name.trim(),
      icon_fallback: form.value.icon_fallback || form.value.name.slice(0, 3),
      provider_type: form.value.provider_type,
      base_url: form.value.base_url || null,
      api_key_plain: form.value.api_key_plain || null,
      models: form.value.models_default ? { default: form.value.models_default } : null,
      notes: form.value.notes || null,
      icon_path,
    })
  } catch (e) {
    console.error('[ProviderForm] auto-save failed:', e)
  }
}
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
  min-height: 0;
}

.template-section {
  margin-bottom: 16px;
}
.template-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
}
body.dark .section-title { color: #ccc; }
.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
  gap: 12px;
}
.tpl-card {
  border: 1px solid #e8e8e8;
  border-radius: 10px;
  padding: 12px 8px 10px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  background: #f5f5f5;
  display: flex;
  flex-direction: column;
  align-items: center;
}
body.dark .tpl-card { background: #333; border-color: #444; }
.tpl-card:hover { border-color: #d77757; box-shadow: 0 4px 12px rgba(215,119,87,0.2); transform: translateY(-2px); }
.tpl-card.selected { border-color: #d77757; background: #faf3eb; }
body.dark .tpl-card.selected { background: #2a2018; }
.tpl-icon-wrap {
  width: 48px; height: 48px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
  background: transparent;
}
.tpl-icon-img { width: 32px; height: 32px; object-fit: contain; }
.tpl-icon { font-size: 18px; }
.tpl-name { font-weight: 600; font-size: 12px; color: #333; text-align: center; }
body.dark .tpl-name { color: #ccc; }
.tpl-desc { font-size: 10px; color: #999; margin-top: 2px; text-align: center; }
.tpl-badge {
  position: absolute;
  top: -4px; right: -4px;
  font-size: 9px;
  color: #fff;
  padding: 2px 5px;
  border-radius: 6px;
  font-weight: 600;
}
.tpl-delete {
  position: absolute;
  top: 4px; left: 4px;
  font-size: 14px;
}
.template-divider {
  margin: 12px 0 8px;
  text-align: center;
  font-size: 12px;
  color: #999;
  border-bottom: 1px dashed #ddd;
}
body.dark .template-divider { border-bottom-color: #444; color: #888; }
</style>