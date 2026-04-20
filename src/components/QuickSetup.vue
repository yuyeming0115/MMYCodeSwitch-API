<template>
  <div class="page">
    <header class="page-header">
      <n-button text size="large" @click="emit('back')">←</n-button>
      <span class="page-title">{{ t('quick_setup') }}</span>
      <span class="page-desc">{{ t('quick_setup_desc') }}</span>
    </header>

    <div class="page-content">
      <!-- Step 1: 选择平台 -->
      <div v-if="step === 1" class="step-content">
        <div class="template-grid">
          <div
            v-for="tpl in templates"
            :key="tpl.id"
            class="tpl-card"
            :class="{ selected: selectedId === tpl.id }"
            @click="selectTemplate(tpl)"
          >
            <div class="tpl-icon-wrap" :style="{ background: tpl.color + '18', borderColor: tpl.color + '40' }">
              <img v-if="tpl.builtinIcon" :src="`/${tpl.builtinIcon}`" class="tpl-icon-img" />
              <span v-else class="tpl-icon">{{ tpl.icon }}</span>
            </div>
            <div class="tpl-name">{{ tpl.name }}</div>
            <div class="tpl-desc">{{ tpl.description }}</div>
            <div v-if="tpl.badge" class="tpl-badge" :style="{ background: tpl.color }">{{ tpl.badge }}</div>
          </div>
        </div>
      </div>

      <!-- Step 2: 填写 Key + 确认配置 -->
      <div v-else class="step-content">
        <div v-if="currentTpl" class="confirm-panel">
          <div class="tpl-header">
            <img v-if="currentTpl.builtinIcon" :src="`/${currentTpl.builtinIcon}`" class="tpl-icon-lg" />
            <span v-else class="tpl-icon-lg">{{ currentTpl.icon }}</span>
            <span class="tpl-title">{{ currentTpl.name }}</span>
          </div>

          <n-alert v-if="currentTpl.helpUrl" type="info" :bordered="false" style="margin-bottom:16px">
            配置方法请查看：
            <n-button text type="primary" @click="openHelp(currentTpl.helpUrl!)">{{ currentTpl.helpUrl }}</n-button>
          </n-alert>

          <n-form label-placement="left" label-width="100px">
            <!-- API Key（唯一必填） -->
            <n-form-item label="API Key" required>
              <n-input
                v-model:value="apiKey"
                :placeholder="currentTpl.keyPlaceholder || '输入你的 API Key'"
                show-password-on="click"
                type="password"
                style="flex:1"
              />
            </n-form-item>

            <!-- 多 Base URL 选择 -->
            <n-form-item v-if="currentTpl.baseUrls && currentTpl.baseUrls.length > 1" label="接口协议">
              <n-radio-group v-model:value="selectedBaseUrlIndex">
                <n-space>
                  <n-radio v-for="(url, idx) in currentTpl.baseUrls" :key="idx" :value="idx">
                    <span>{{ url.label }}</span>
                    <span v-if="url.hint" style="font-size:11px;color:#999;margin-left:4px">({{ url.hint }})</span>
                  </n-radio>
                </n-space>
              </n-radio-group>
              <div v-if="protocolHint" style="font-size:11px;color:#e89834;margin-top:4px">{{ protocolHint }}</div>
            </n-form-item>

            <!-- 自定义名称 -->
            <n-form-item label="显示名称">
              <n-input v-model:value="customName" :placeholder="'默认: ' + currentTpl.name" />
            </n-form-item>

            <!-- 默认模型 -->
            <n-form-item label="默认模型">
              <div style="flex:1;display:flex;gap:8px;align-items:center">
                <n-select
                  v-model:value="selectedModel"
                  :options="modelOptions"
                  :placeholder="fetchingModels ? '正在同步模型列表...' : '选择默认模型'"
                  filterable
                  style="flex:1"
                  :loading="fetchingModels"
                />
                <n-button size="small" type="tertiary" :disabled="!apiKey || fetchingModels" @click="fetchModels" title="从平台实时同步模型列表">
                  🔄
                </n-button>
              </div>
              <div v-if="fetchedModels.length > 0" style="font-size:11px;color:#18a058;margin-top:4px">
                已从平台同步 {{ fetchedModels.length }} 个模型
              </div>
            </n-form-item>

            <!-- 备注 -->
            <n-form-item label="备注">
              <n-input v-model:value="notes" type="textarea" :rows="2" placeholder="可选备注信息" />
            </n-form-item>
          </n-form>

          <!-- 配置预览 -->
          <div class="preview-section">
            <div class="preview-title">📋 生成的配置预览</div>
            <div class="preview-list">
              <div class="preview-row"><span class="pv-key">名称</span><span class="pv-val">{{ effectiveName }}</span></div>
              <div class="preview-row"><span class="pv-key">类型</span><span class="pv-val">API</span></div>
              <div class="preview-row"><span class="pv-key">Base URL</span><span class="pv-val mono">{{ effectiveBaseUrl }}</span></div>
              <div class="preview-row"><span class="pv-key">API Key</span><span class="pv-val">{{ apiKey ? '••••' + apiKey.slice(-4) : '未填写' }}</span></div>
              <div class="preview-row"><span class="pv-key">模型</span><span class="pv-val mono">{{ selectedModel || currentTpl.models[0] || '(未选)' }}</span></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <footer class="page-footer">
      <n-button v-if="step === 2" size="large" @click="step = 1">← 返回选择</n-button>
      <n-button v-else size="large" @click="emit('back')">← 返回</n-button>
      <n-button v-if="step === 1" type="primary" size="large" :disabled="!selectedId" @click="step = 2">下一步 →</n-button>
      <n-button v-else type="primary" size="large" :disabled="!apiKey" :loading="saving" @click="doCreate">✨ 一键生成</n-button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/app'
import { useMessage } from 'naive-ui'

const { t } = useI18n()

interface BaseUrlOption {
  label: string
  value: string
  hint?: string
  protocolHint?: string
}

interface Template {
  id: string
  name: string
  icon: string
  color: string
  description: string
  badge?: string
  keyPlaceholder?: string
  helpUrl?: string
  baseUrls: BaseUrlOption[]
  models: string[]
  iconFallback: string
  notesHint?: string
  builtinIcon?: string
}

const emit = defineEmits<{ back: []; done: [] }>()
const store = useAppStore()
const msg = useMessage()

const templates: Template[] = [
  {
    id: 'dashscope',
    name: '阿里云百炼',
    icon: '☁️',
    color: '#FF6A00',
    description: 'DashScope · 多模型',
    badge: '推荐',
    keyPlaceholder: 'sk-xxxxxxxx',
    helpUrl: 'https://bailian.console.aliyun.com/',
    baseUrls: [
      { label: 'Anthropic 兼容', value: 'https://coding.dashscope.aliyuncs.com/apps/anthropic', hint: 'Claude Code', protocolHint: '适用于 Claude Code、Cursor 等 Anthropic 协议工具' },
      { label: 'OpenAI 兼容', value: 'https://coding.dashscope.aliyuncs.com/v1', hint: 'LobeChat 等', protocolHint: '适用于 OpenAI SDK 兼容的客户端' },
    ],
    models: ['qwen3.6-plus', 'qwen3.5-plus', 'qwen3-max-2026-01-23', 'qwen3-coder-next', 'glm-5', 'glm-4.7', 'kimi-k2.5', 'MiniMax-M2.5'],
    iconFallback: '百炼',
    notesHint: '阿里云百炼 DashScope 平台',
    builtinIcon: 'icons/dashscope.svg',
  },
  {
    id: 'minimax',
    name: 'MiniMax',
    icon: '〰️',
    color: '#FF4D4F',
    description: 'MiniMax M2.5',
    keyPlaceholder: 'eyJxxxxxx',
    baseUrls: [{ label: 'API', value: 'https://api.minimax.chat/v1' }],
    models: ['MiniMax-M2.5', 'MiniMax-M2.1'],
    iconFallback: 'MM',
    builtinIcon: 'icons/minimax.svg',
  },
  {
    id: 'zhipu',
    name: '智谱 Zhipu',
    icon: '🅉',
    color: '#4D6BFE',
    description: 'GLM 系列',
    keyPlaceholder: 'xxxx.xxxx',
    baseUrls: [{ label: 'OpenAI 兼容', value: 'https://open.bigmodel.cn/api/paas/v4' }],
    models: ['glm-5', 'glm-4.7', 'glm-4-flash', 'glm-4-air'],
    iconFallback: '智谱',
    builtinIcon: 'icons/zhipu.svg',
  },
  {
    id: 'kimi',
    name: 'Kimi (月之暗面)',
    icon: '🅺',
    color: '#7C3AED',
    description: 'Moonshot k2.5',
    keyPlaceholder: 'sk-xxxxx',
    baseUrls: [{ label: 'API', value: 'https://api.moonshot.cn/v1' }],
    models: ['kimi-k2.5', 'kimi-k2-0711-preview', 'moonshot-v1-8k', 'moonshot-v1-32k'],
    iconFallback: 'K',
    builtinIcon: 'icons/kimi.svg',
  },
  {
    id: 'huoshan',
    name: '火山引擎',
    icon: '🔥',
    color: '#F25919',
    description: '豆包 Doubao',
    keyPlaceholder: 'xxxxx',
    baseUrls: [{ label: 'API', value: 'https://ark.cn-beijing.volces.com/api/v3' }],
    models: ['doubao-1-5-pro', 'doubao-1-5-lite'],
    iconFallback: '火',
    builtinIcon: 'icons/huoshan.svg',
  },
  {
    id: 'tencent',
    name: '腾讯云',
    icon: '☁️',
    color: '#0066FF',
    description: '混元 Hunyuan',
    keyPlaceholder: 'sk-xxxxx',
    baseUrls: [{ label: 'OpenAI 兼容', value: 'https://api.hunyuan.cloud.tencent.com/v1' }],
    models: ['hunyuan-turbos-latest', 'hunyuan-standard'],
    iconFallback: '腾',
    builtinIcon: 'icons/tencent.svg',
  },
]

const step = ref(1)
const selectedId = ref('')
const apiKey = ref('')
const customName = ref('')
const selectedModel = ref<string | null>(null)
const selectedBaseUrlIndex = ref(0)
const notes = ref('')
const saving = ref(false)
const fetchedModels = ref<string[]>([])
const fetchingModels = ref(false)

watch(() => step.value, (val) => {
  if (val === 1) {
    selectedId.value = ''
    apiKey.value = ''
    customName.value = ''
    selectedModel.value = null
    selectedBaseUrlIndex.value = 0
    notes.value = ''
    fetchedModels.value = []
  }
})

const currentTpl = computed(() => templates.find(t => t.id === selectedId.value))
const effectiveName = computed(() => customName.value.trim() || currentTpl.value?.name || '')
const effectiveBaseUrl = computed(() => {
  const urls = currentTpl.value?.baseUrls
  if (!urls || urls.length === 0) return ''
  return urls[selectedBaseUrlIndex.value]?.value ?? urls[0].value
})
const modelOptions = computed(() => {
  const source = fetchedModels.value.length > 0 ? fetchedModels.value : (currentTpl.value?.models ?? [])
  return source.map(m => ({ label: m, value: m }))
})
const protocolHint = computed(() => {
  const urls = currentTpl.value?.baseUrls
  if (!urls || urls.length === 0) return ''
  return urls[selectedBaseUrlIndex.value]?.protocolHint ?? ''
})

function selectTemplate(tpl: Template) {
  selectedId.value = tpl.id
  apiKey.value = ''
  customName.value = ''
  selectedModel.value = null
  selectedBaseUrlIndex.value = 0
  notes.value = tpl.notesHint || ''
  fetchedModels.value = []
  if (tpl.models.length > 0) {
    selectedModel.value = tpl.models[0]
  }
}

async function fetchModels() {
  if (!apiKey.value || !currentTpl.value) return
  fetchingModels.value = true
  try {
    const baseUrl = 'https://coding.dashscope.aliyuncs.com/v1'
    const resp = await fetch(`${baseUrl}/models`, {
      headers: { Authorization: `Bearer ${apiKey.value}` },
      signal: AbortSignal.timeout(10000),
    })
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
    const data = await resp.json()
    const ids: string[] = (data.data ?? []).map((m: { id?: string }) => m.id).filter(Boolean).sort()
    if (ids.length > 0) {
      fetchedModels.value = ids
      if (!selectedModel.value || !ids.includes(selectedModel.value)) {
        selectedModel.value = ids[0]
      }
      msg.success(`已同步 ${ids.length} 个模型`)
    } else {
      msg.warning('未获取到模型列表')
    }
  } catch (e) {
    msg.warning('模型同步失败')
  } finally {
    fetchingModels.value = false
  }
}

function openHelp(url: string) {
  invoke('tauri', { __tauriModule: 'shell', message: { cmd: 'open', payload: url } })
}

async function doCreate() {
  if (!apiKey.value) { msg.error('请输入 API Key'); return }
  if (!currentTpl.value) return

  saving.value = true
  try {
    const tpl = currentTpl.value
    const input = {
      id: null,
      name: effectiveName.value,
      icon_fallback: tpl.iconFallback,
      provider_type: 'api',
      base_url: effectiveBaseUrl.value,
      api_key_plain: apiKey.value,
      models: selectedModel.value ? { default: selectedModel.value } : null,
      notes: notes.value || null,
      icon_path: tpl.builtinIcon || null,
    }
    await store.upsertProvider(input)
    msg.success(`✅ 已生成「${effectiveName.value}」配置`)
    emit('done')
  } catch (e) {
    msg.error('保存失败: ' + e)
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
.page-desc { font-size: 12px; color: #888; margin-left: auto; }
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

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
}
.tpl-card {
  border: 2px solid #e8e8e8;
  border-radius: 14px;
  padding: 16px 12px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  background: #fff;
  display: flex;
  flex-direction: column;
  align-items: center;
}
body.dark .tpl-card { background: #2a2a2a; border-color: #444; }
.tpl-card:hover { border-color: #18a058; box-shadow: 0 4px 16px rgba(24,160,88,0.15); }
.tpl-card.selected { border-color: #18a058; background: #f0faf5; }
body.dark .tpl-card.selected { background: #1a3a28; }
.tpl-icon-wrap {
  width: 52px; height: 52px;
  border-radius: 14px;
  border: 1.5px solid;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
}
.tpl-icon-img { width: 32px; height: 32px; object-fit: contain; }
.tpl-icon { font-size: 24px; }
.tpl-name { font-weight: 600; font-size: 13px; color: #333; text-align: center; }
body.dark .tpl-name { color: #ccc; }
.tpl-desc { font-size: 10px; color: #999; margin-top: 2px; text-align: center; }
.tpl-badge {
  position: absolute;
  top: -5px; right: -5px;
  font-size: 9px;
  color: #fff;
  padding: 2px 6px;
  border-radius: 8px;
  font-weight: 600;
}

.confirm-panel { }
.tpl-header { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; }
.tpl-icon-lg { width: 36px; height: 36px; object-fit: contain; font-size: 28px; }
.tpl-title { font-size: 18px; font-weight: 700; }

.preview-section {
  margin-top: 16px;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;
}
body.dark .preview-section { border-color: #444; }
.preview-title {
  background: #f5f5f5;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  color: #555;
}
body.dark .preview-title { background: #333; color: #aaa; }
.preview-list { padding: 4px 0; }
.preview-row { display: flex; padding: 6px 12px; font-size: 13px; }
.pv-key { width: 80px; color: #888; font-weight: 500; flex-shrink: 0; }
.pv-val { color: #333; word-break: break-all; }
body.dark .pv-val { color: #ccc; }
.mono { font-family: 'Cascadia Code', Consolas, monospace; font-size: 12px; }
</style>