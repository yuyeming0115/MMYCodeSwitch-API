<template>
  <div class="page">
    <header class="page-header">
      <n-button text size="large" @click="emit('back')">←</n-button>
      <span class="page-title">{{ t('usage_stats') }}</span>
      <n-button text size="large" @click="loadStats" style="margin-left: auto">
        <template #icon><n-icon><refresh-outline-icon /></n-icon></template>
      </n-button>
    </header>

    <div class="page-content" v-if="stats">
      <!-- 平台选择 -->
      <div class="platform-bar">
        <div class="platform-label">{{ t('select_platform') }}</div>
        <div class="platform-chips">
          <button
            :class="['chip', { active: selectedPlatform === 'all' }]"
            @click="selectedPlatform = 'all'; loadStats()"
          >{{ t('all_platforms') }}</button>
          <button
            v-for="p in platforms" :key="p"
            :class="['chip', { active: selectedPlatform === p }]"
            @click="selectedPlatform = p; loadStats()"
          >{{ p }}</button>
        </div>
      </div>

      <!-- Tab 栏 + 时间筛选 -->
      <div class="tab-bar">
        <button
          :class="['tab-btn', { active: activeTab === 'overview' }]"
          @click="activeTab = 'overview'; refreshChart()"
        >{{ t('overview') }}</button>
        <button
          :class="['tab-btn', { active: activeTab === 'models' }]"
          @click="activeTab = 'models'; refreshChart()"
        >{{ t('models') }}</button>
        <div class="tab-spacer"></div>
        <button
          :class="['time-btn', { active: period === 'all' }]"
          @click="period = 'all'; loadStats()"
        >{{ t('period_all') }}</button>
        <button
          :class="['time-btn', { active: period === '30d' }]"
          @click="period = '30d'; loadStats()"
        >{{ t('period_30d') }}</button>
        <button
          :class="['time-btn', { active: period === '7d' }]"
          @click="period = '7d'; loadStats()"
        >{{ t('period_7d') }}</button>
      </div>

      <!-- 概览 -->
      <div v-show="activeTab === 'overview'" class="tab-content">
        <div class="stats-grid">
          <div class="stat-card">
            <div class="stat-label">{{ t('sessions') }}</div>
            <div class="stat-value">{{ stats.sessions }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('messages') }}</div>
            <div class="stat-value">{{ formatNumber(stats.messages) }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('total_tokens') }}</div>
            <div class="stat-value">{{ formatTokens(stats.input_tokens + stats.output_tokens) }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('active_days') }}</div>
            <div class="stat-value">{{ stats.active_days }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('current_streak') }}</div>
            <div class="stat-value">{{ stats.current_streak }} {{ t('days') }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('longest_streak') }}</div>
            <div class="stat-value">{{ stats.longest_streak }} {{ t('days') }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('peak_hour') }}</div>
            <div class="stat-value">{{ formatHour(stats.peak_hour) }}</div>
          </div>
          <div class="stat-card">
            <div class="stat-label">{{ t('favorite_model') }}</div>
            <div class="stat-value favorite-model">{{ stats.favorite_model }}</div>
          </div>
        </div>

        <!-- 热力图 -->
        <div class="heatmap-section">
          <div class="heatmap" :style="{ gridTemplateColumns: `repeat(${heatmapCols}, 1fr)` }">
            <div
              v-for="(day, idx) in heatmapDays"
              :key="idx"
              class="heatmap-cell"
              :style="getHeatStyle(day.total)"
              :title="`${day.date}: ${formatTokens(day.total)} tokens`"
            ></div>
          </div>
          <div class="heatmap-footer">
            <span class="heatmap-legend">{{ t('heatmap_legend') }}</span>
            <span class="heatmap-total">{{ t('period_total_tokens', { tokens: formatTokens(stats.input_tokens + stats.output_tokens) }) }}</span>
          </div>
        </div>
      </div>

      <!-- 模型 -->
      <div v-show="activeTab === 'models'" class="tab-content models-content">
        <div ref="chartRef" class="chart-container"></div>
        <div class="model-legend" v-if="stats.model_data.length > 0">
          <div v-for="m in stats.model_data" :key="m.model" class="model-legend-item">
            <span class="model-dot" :style="{ background: getModelColor(m.model) }"></span>
            <span class="model-name">{{ m.model }}</span>
            <span class="model-detail">
              {{ t('input') }} {{ formatTokens(m.input_tokens) }} · {{ t('output') }} {{ formatTokens(m.output_tokens) }}
            </span>
            <span class="model-pct">{{ m.percentage.toFixed(1) }}%</span>
          </div>
        </div>
        <p v-else class="empty-model-hint">{{ t('usage_no_data') }}</p>
      </div>
    </div>

    <div class="page-content empty" v-else>
      <p class="empty-hint">{{ t('usage_no_data') }}</p>
    </div>

    <footer class="page-footer">
      <n-button size="large" @click="emit('back')">{{ t('cancel') }}</n-button>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { RefreshOutline as RefreshOutlineIcon } from '@vicons/ionicons5'
import { useMessage } from 'naive-ui'
import * as echarts from 'echarts'
import { getCurrentWindow } from '@tauri-apps/api/window'

const { t, locale } = useI18n()
const msg = useMessage()
const emit = defineEmits<{ back: [] }>()
const appWindow = getCurrentWindow()

interface UsageStats {
  sessions: number
  messages: number
  input_tokens: number
  output_tokens: number
  active_days: number
  current_streak: number
  longest_streak: number
  peak_hour: number
  favorite_model: string
  daily_data: { date: string; input_tokens: number; output_tokens: number; messages: number }[]
  model_data: { model: string; input_tokens: number; output_tokens: number; percentage: number }[]
}

const stats = ref<UsageStats | null>(null)
const activeTab = ref('overview')
const period = ref('all')
const selectedPlatform = ref('all')
const chartRef = ref<HTMLElement | null>(null)
let chartInstance: echarts.ECharts | null = null

// 平台列表（从模型名推断）
const platforms = computed(() => {
  if (!stats.value) return []
  const set = new Set<string>()
  for (const m of stats.value.model_data) {
    const model = m.model.toLowerCase()
    if (model.startsWith('claude')) set.add('Claude')
    else if (model.startsWith('glm')) set.add('智谱')
    else if (model.startsWith('qwen')) set.add('阿里百炼')
    else if (model.startsWith('deepseek')) set.add('DeepSeek')
    else if (model.startsWith('gpt')) set.add('OpenAI')
    else set.add('其他')
  }
  return Array.from(set)
})

// 模型颜色映射（蓝色系）
const modelColors: Record<string, string> = {}
const colorPalette = ['#2979ff', '#448aff', '#64b5f6', '#90caf9', '#bbdefb', '#e3f2fd']
let colorIndex = 0

function getModelColor(model: string): string {
  if (!modelColors[model]) {
    modelColors[model] = colorPalette[colorIndex % colorPalette.length]
    colorIndex++
  }
  return modelColors[model]
}

async function loadStats() {
  try {
    stats.value = await invoke<UsageStats>('get_usage_stats', { period: period.value })
    await nextTick()
    if (activeTab.value === 'models') {
      refreshChart()
    }
  } catch (e) {
    msg.error(t('usage_load_failed', { msg: e instanceof Error ? e.message : String(e) }))
  }
}

function formatNumber(n: number): string {
  return n.toLocaleString()
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return n.toString()
}

function formatHour(h: number): string {
  const hourStr = h.toString().padStart(2, '0') + ':00'
  if (locale.value === 'zh') {
    if (h >= 5 && h < 12) return `上午 ${h} 点`
    if (h === 12) return `中午 12 点`
    if (h >= 13 && h < 18) return `下午 ${h - 12} 点`
    if (h >= 18 && h < 22) return `晚上 ${h - 12} 点`
    return `凌晨 ${h} 点`
  }
  return hourStr
}

// 热力图 — 蓝色渐变
const heatmapCols = 14

const heatmapDays = computed(() => {
  if (!stats.value) return []
  const map: Record<string, number> = {}
  for (const d of stats.value.daily_data) {
    map[d.date] = d.input_tokens + d.output_tokens
  }
  const now = new Date()
  const days = period.value === '7d' ? 7 : period.value === '30d' ? 30 : 90
  const result: { date: string; total: number }[] = []
  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(now)
    d.setDate(d.getDate() - i)
    const dateStr = d.toISOString().split('T')[0]
    result.push({ date: dateStr, total: map[dateStr] || 0 })
  }
  return result
})

function getHeatStyle(total: number): Record<string, string> {
  if (total === 0) return { background: 'var(--heat-0)' }
  if (total < 50_000) return { background: 'var(--heat-1)' }
  if (total < 200_000) return { background: 'var(--heat-2)' }
  if (total < 500_000) return { background: 'var(--heat-3)' }
  return { background: 'var(--heat-4)' }
}

// 初始化 / 刷新图表
function refreshChart() {
  if (!stats.value || stats.value.daily_data.length === 0) return

  nextTick(() => {
    if (!chartRef.value) return

    if (!chartInstance) {
      chartInstance = echarts.init(chartRef.value)
    }

    const container = chartRef.value
    if (container.clientWidth === 0) {
      container.style.width = '100%'
      container.style.height = '350px'
    }
    chartInstance.resize()

    const { daily_data, model_data } = stats.value!

    const dates = daily_data.map(d => {
      const parts = d.date.split('-')
      return locale.value === 'zh' ? `${parseInt(parts[1])}月${parseInt(parts[2])}日` : `${parts[1]}-${parts[2]}`
    })

    const modelRatio: Record<string, number> = {}
    let grandTotal = 0
    for (const m of model_data) {
      grandTotal += m.input_tokens + m.output_tokens
    }
    for (const m of model_data) {
      modelRatio[m.model] = grandTotal > 0 ? (m.input_tokens + m.output_tokens) / grandTotal : 0
    }

    const series = model_data.map(m => ({
      name: m.model,
      type: 'bar',
      stack: 'total',
      emphasis: { focus: 'series' },
      data: daily_data.map(d =>
        Math.round((d.input_tokens + d.output_tokens) * modelRatio[m.model])
      ),
      itemStyle: { color: getModelColor(m.model) },
    }))

    chartInstance.setOption({
      tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
      legend: { show: false },
      grid: { left: '3%', right: '4%', bottom: '3%', top: '3%', containLabel: true },
      xAxis: { type: 'category', data: dates, axisLabel: { fontSize: 11 } },
      yAxis: {
        type: 'value',
        axisLabel: { formatter: (v: number) => formatTokens(v) },
      },
      series,
    })
  })
}

onMounted(async () => {
  await loadStats()

  // 监听窗口大小变化，自动调整图表
  await appWindow.onResized(() => {
    if (chartInstance && activeTab.value === 'models') {
      chartInstance.resize()
    }
  })
})

onUnmounted(() => {
  if (chartInstance) {
    chartInstance.dispose()
    chartInstance = null
  }
})
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
.page-content.empty {
  display: flex;
  align-items: center;
  justify-content: center;
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

/* 平台选择 */
.platform-bar { margin-bottom: 12px; }
.platform-label { font-size: 12px; color: #999; margin-bottom: 6px; }
body.dark .platform-label { color: #666; }
.platform-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.chip {
  padding: 6px 14px;
  border: 1px solid #e0e0e0;
  border-radius: 20px;
  background: #fff;
  color: #555;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}
body.dark .chip {
  border-color: #444;
  background: #2a2a2a;
  color: #aaa;
}
.chip:hover { border-color: #2979ff; color: #2979ff; }
body.dark .chip:hover { border-color: #448aff; color: #448aff; }
.chip.active {
  background: #2979ff;
  border-color: #2979ff;
  color: #fff;
}
body.dark .chip.active {
  background: #2979ff;
  border-color: #2979ff;
  color: #fff;
}

/* Tab 栏 */
.tab-bar {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
  padding: 4px;
  background: #f0f0f0;
  border-radius: 8px;
}
body.dark .tab-bar { background: #333; }
.tab-btn {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: #666;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}
body.dark .tab-btn { color: #999; }
.tab-btn:hover { background: rgba(0,0,0,0.05); }
body.dark .tab-btn:hover { background: rgba(255,255,255,0.05); }
.tab-btn.active {
  background: #fff;
  color: #333;
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
body.dark .tab-btn.active {
  background: #2a2a2a;
  color: #fff;
}
.tab-spacer { flex: 1; }

/* 时间按钮 */
.time-btn {
  padding: 6px 14px;
  border: none;
  border-radius: 16px;
  background: transparent;
  color: #666;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}
body.dark .time-btn { color: #999; }
.time-btn:hover { background: rgba(0,0,0,0.05); }
body.dark .time-btn:hover { background: rgba(255,255,255,0.05); }
.time-btn.active {
  background: #e3f2fd;
  color: #1565c0;
  font-weight: 600;
}
body.dark .time-btn.active {
  background: #1a3a5a;
  color: #64b5f6;
}

.tab-content { min-height: 200px; }

/* 统计卡片 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}
.stat-card {
  padding: 12px;
  background: #f5f5f5;
  border-radius: 6px;
}
body.dark .stat-card { background: #2a2a2a; }
.stat-label { font-size: 12px; color: #888; margin-bottom: 4px; }
body.dark .stat-label { color: #999; }
.stat-value { font-size: 20px; font-weight: 700; }
.favorite.model { font-size: 13px; color: #666; word-break: break-all; }
body.dark .favorite.model { color: #aaa; }

/* 热力图 — 蓝色渐变 */
.heatmap-section { margin-top: 16px; }
.heatmap {
  display: grid;
  gap: 3px;
}
.heatmap-cell {
  aspect-ratio: 1;
  border-radius: 3px;
}
.heatmap-footer {
  display: flex;
  justify-content: space-between;
  margin-top: 8px;
  font-size: 12px;
  color: #999;
}
body.dark .heatmap-footer { color: #666; }

/* 图表 */
.chart-container {
  width: 100%;
  height: 350px;
  margin-bottom: 16px;
}

/* 模型图例 */
.model-legend { display: flex; flex-direction: column; gap: 6px; }
.model-legend-item {
  display: grid;
  grid-template-columns: 10px 180px 1fr 60px;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f5f5f5;
  border-radius: 6px;
}
body.dark .model-legend-item { background: #2a2a2a; }
.model-dot { width: 10px; height: 10px; border-radius: 2px; }
.model-name { font-weight: 600; font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.model-detail { font-size: 12px; color: #888; }
body.dark .model-detail { color: #999; }
.model-pct { font-weight: 600; font-size: 13px; color: #18a058; text-align: right; }

.empty-model-hint { text-align: center; color: #999; padding: 40px 0; }
body.dark .empty-model-hint { color: #666; }
</style>

<style>
/* 全局热力图颜色变量 */
:root {
  --heat-0: #ebedf0;
  --heat-1: #c6e4f7;
  --heat-2: #7ac8f0;
  --heat-3: #3a9bd5;
  --heat-4: #1a6fb5;
}
body.dark {
  --heat-0: #2a2a2a;
  --heat-1: #0e3a5a;
  --heat-2: #1565a0;
  --heat-3: #2979ff;
  --heat-4: #448aff;
}
</style>
