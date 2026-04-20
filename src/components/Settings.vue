<template>
  <n-modal v-model:show="show" preset="card" :title="t('settings')" style="width:500px">
    <n-form label-placement="left" label-width="80px">
      <n-form-item :label="t('language')">
        <n-radio-group :value="store.config.language" @update:value="setLang">
          <n-radio value="zh">中文</n-radio>
          <n-radio value="en">English</n-radio>
        </n-radio-group>
      </n-form-item>
      <n-divider>{{ t('instance_management') }}</n-divider>
      <div v-for="(inst, idx) in store.config.instances" :key="inst.id" style="margin-bottom:8px">
        <n-space align="center">
          <n-input v-model:value="inst.name" style="width:120px" />
          <n-input v-model:value="inst.config_dir" style="width:220px" placeholder="~/.claude" />
          <n-button text type="error" :disabled="store.config.instances.length <= 1" @click="removeInstance(idx)">✕</n-button>
        </n-space>
      </div>
      <n-button dashed block @click="addInstance">+ {{ t('add_instance') }}</n-button>
    </n-form>
    <template #footer>
      <n-space justify="end">
        <n-button @click="show = false">{{ t('cancel') }}</n-button>
        <n-button type="primary" @click="save">{{ t('save') }}</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../stores/app'
import { useMessage } from 'naive-ui'
import { i18n } from '../i18n'

const { t } = useI18n()
const store = useAppStore()
const msg = useMessage()
const show = defineModel<boolean>('show', { default: false })

function setLang(lang: string) {
  store.config.language = lang
  i18n.global.locale.value = lang as 'zh' | 'en'
}

function addInstance() {
  store.config.instances.push({
    id: `instance_${Date.now()}`,
    name: '新实例',
    config_dir: '',
    active_provider_id: undefined,
  })
}

function removeInstance(idx: number) {
  store.config.instances.splice(idx, 1)
}

async function save() {
  await store.saveConfig(store.config)
  msg.success(t('save_success'))
  show.value = false
}
</script>
