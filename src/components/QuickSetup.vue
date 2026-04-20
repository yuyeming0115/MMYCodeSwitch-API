<template>
  <n-modal v-model:show="show" preset="card" title="快速配置" style="width:580px">
    <!-- Step 1: 选择平台 -->
    <div v-if="step === 1" class="step-content">
      <n-p>选择你要配置的平台，只需填写 API Key 即可自动生成完整配置：</n-p>
      <div class="template-grid">
        <div
          v-for="tpl in templates"
          :key="tpl.id"
          class="tpl-card"
          :class="{ selected: selectedId === tpl.id }"
          @click="selectTemplate(tpl)"
        >
          <div class="tpl-icon-wrap" :style="{ background: tpl.color + '18', borderColor: tpl.color + '40' }">
            <span class="tpl-icon">{{ tpl.icon }}</span>
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
          <span class="tpl-icon-lg">{{ currentTpl.icon }}</span>
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
        <n-collapse-transition :show="true">
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
        </n-collapse-transition>
      </div>
    </div>

    <!-- Footer -->
    <template #footer>
      <n-space justify="space-between">
        <n-button v-if="step === 2" @click="step = 1">← 返回选择</n-button>
        <div v-else />
        <n-space>
          <n-button @click="show = false">取消</n-button>
          <n-button v-if="step === 1" type="primary" :disabled="!selectedId" @click="step = 2">下一步 →</n-button>
          <n-button v-else type="primary" :disabled="!apiKey" :loading="saving" @click="doCreate">✨ 一键生成配置</n-button>
        </n-space>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../stores/app'
import { useMessage } from 'naive-ui'

interface BaseUrlOption {
  label: string
  value: string
  hint?: string        // 协议适用场景提示
  protocolHint?: string // 选中后显示的详细提示
}

interface Template {
  id: string
  name: string
  icon: string          // emoji 或 SVG 字符
  color: string         // 品牌主色
  description: string
  badge?: string
  keyPlaceholder?: string
  helpUrl?: string
  baseUrls: BaseUrlOption[]
  models: string[]
  iconFallback: string
  notesHint?: string
  /** 内置图标资源路径（相对于 public/ 目录，通过 asset://localhost 加载） */
  builtinIcon?: string
}

const show = defineModel<boolean>('show', { default: false })
const emit = defineEmits(['done'])
const store = useAppStore()
const msg = useMessage()

// ── 内置平台模板（参考 JCode 风格） ───────────────────────────
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
      { label: 'OpenAI 兼容', value: 'https://coding.dashscope.aliyuncs.com/v1', hint: 'LobeChat 等', protocolHint: '适用于 OpenAI SDK 兼容的客户端（ChatGPT-Next-Web、LobeChat 等）' },
    ],
    models: ['qwen3.6-plus', 'qwen3.5-plus', 'qwen3-max-2026-01-23', 'qwen3-coder-next', 'qwen3-coder-plus', 'glm-5', 'glm-4.7', 'kimi-k2.5', 'MiniMax-M2.5'],
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

// 可以继续扩展更多平台模板：
// {
//   id: 'openai-official',
//   name: 'OpenAI 官方',
//   icon: '🟢',
//   description: 'GPT / o 系列',
//   baseUrls: [{ label: '官方', value: 'https://api.openai.com/v1' }],
//   models: ['gpt-4o', 'o3-mini', 'o4-mini'],
//   iconFallback: 'OA',
// },

const step = ref(1)
const selectedId = ref('')
const apiKey = ref('')
const customName = ref('')
const selectedModel = ref<string | null>(null)
const selectedBaseUrlIndex = ref(0)
const notes = ref('')
const saving = ref(false)

// 动态模型列表（从 API 拉取后覆盖静态 fallback）
const fetchedModels = ref<string[]>([])
const fetchingModels = ref(false)

// ── 打开时重置到 Step 1 ─────────────────────────────────────
watch(show, (val) => {
  if (val) {
    // 每次打开弹窗都重置回平台选择页
    step.value = 1
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

// 当前选中协议的提示文字
const protocolHint = computed(() => {
  const urls = currentTpl.value?.baseUrls
  if (!urls || urls.length === 0) return ''
  return urls[selectedBaseUrlIndex.value]?.protocolHint ?? ''
})

function selectTemplate(tpl: Template) {
  selectedId.value = tpl.id
  // 重置表单
  apiKey.value = ''
  customName.value = ''
  selectedModel.value = null
  selectedBaseUrlIndex.value = 0
  notes.value = tpl.notesHint || ''
  fetchedModels.value = []   // 重置动态模型列表
  // 自动选中第一个模型
  if (tpl.models.length > 0) {
    selectedModel.value = tpl.models[0]
  }
}

/** 从百炼 API 动态拉取可用模型列表 */
async function fetchModels() {
  if (!apiKey.value || !currentTpl.value) return
  fetchingModels.value = true
  try {
    // 使用 OpenAI 兼容的 /v1/models 端点拉取（百炼支持）
    const baseUrl = 'https://coding.dashscope.aliyuncs.com/v1'
    const resp = await fetch(`${baseUrl}/models`, {
      headers: { Authorization: `Bearer ${apiKey.value}` },
      signal: AbortSignal.timeout(10000),
    })
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
    const data = await resp.json()
    const ids: string[] = (data.data ?? [])
      .map((m: { id?: string }) => m.id)
      .filter(Boolean)
      .sort()
    if (ids.length > 0) {
      fetchedModels.value = ids
      if (!selectedModel.value || !ids.includes(selectedModel.value)) {
        selectedModel.value = ids[0]
      }
      msg.success(`已同步 ${ids.length} 个模型`)
    } else {
      msg.warning('未获取到模型列表，使用内置默认列表')
    }
  } catch (e) {
    msg.warning('模型同步失败，将使用内置默认列表 (' + e + ')')
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
      // 优先使用内置图标（public/icons/xxx.svg），通过 asset://localhost 加载
      icon_path: tpl.builtinIcon || null,
    }
    await store.upsertProvider(input)
    msg.success(`✅ 已生成「${effectiveName.value}」配置`)
    show.value = false
    emit('done')
  } catch (e) {
    msg.error('保存失败: ' + e)
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.step-content { min-height: 300px; }

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 14px;
  margin-top: 12px;
}
.tpl-card {
  border: 2px solid #e8e8e8;
  border-radius: 16px;
  padding: 18px 14px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  background: #fff;
  display: flex;
  flex-direction: column;
  align-items: center;
}
.tpl-card:hover { border-color: #18a058; box-shadow: 0 4px 16px rgba(24,160,88,0.15); transform: translateY(-2px); }
.tpl-card.selected { border-color: #18a058; background: #f0faf5; box-shadow: 0 4px 12px rgba(24,160,88,0.18); }
.tpl-icon-wrap {
  width: 56px; height: 56px;
  border-radius: 16px;
  border: 1.5px solid;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 10px;
  transition: all 0.2s;
}
.tpl-card:hover .tpl-icon-wrap,
.tpl-card.selected .tpl-icon-wrap {
  transform: scale(1.08);
}
.tpl-icon { font-size: 26px; line-height: 1; }
.tpl-name { font-weight: 600; font-size: 13.5px; margin-top: 4px; color: #333; text-align: center; }
.tpl-desc { font-size: 11px; color:#999; margin-top: 2px; text-align: center; }
.tpl-badge {
  position: absolute;
  top: -6px;
  right: -6px;
  font-size: 10px;
  color: #fff;
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.confirm-panel { }
.tpl-header { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; }
.tpl-icon-lg { font-size: 32px; }
.tpl-title { font-size: 20px; font-weight: 700; }

.preview-section {
  margin-top: 16px;
  border: 1px solid #e0e0e0;
  border-radius: 8px;
  overflow: hidden;
}
.preview-title {
  background: #f5f5f5;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 600;
  color: #555;
  border-bottom: 1px solid #e0e0e0;
}
.preview-list { padding: 4px 0; }
.preview-row {
  display: flex;
  padding: 6px 14px;
  font-size: 13px;
}
.pv-key { width: 90px; color: #888; font-weight: 500; flex-shrink: 0; }
.pv-val { color: #333; word-break: break-all; }
.mono { font-family: 'Cascadia Code', 'Fira Code', Consolas, monospace; font-size: 12px; }
</style>
