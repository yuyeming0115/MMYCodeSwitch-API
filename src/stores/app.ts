import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export interface Provider {
  id: string
  name: string
  icon_fallback: string
  provider_type: string
  base_url?: string
  api_key_encrypted?: string
  models?: Record<string, string>
  notes?: string
  icon_path?: string
  created_at: string
  updated_at: string
}

export interface Instance {
  id: string
  name: string
  config_dir: string
  active_provider_id?: string
}

export interface AppConfig {
  language: string
  instances: Instance[]
}

export const useAppStore = defineStore('app', () => {
  const providers = ref<Provider[]>([])
  const config = ref<AppConfig>({ language: 'zh', instances: [] })
  const activeInstanceId = ref('default')

  const activeInstance = () => config.value.instances.find(i => i.id === activeInstanceId.value) ?? config.value.instances[0]

  async function init() {
    await invoke('init_app')
    await loadConfig()
    await loadProviders()
  }

  async function loadConfig() {
    config.value = await invoke<AppConfig>('get_app_config')
    if (config.value.instances.length > 0 && !config.value.instances.find(i => i.id === activeInstanceId.value)) {
      activeInstanceId.value = config.value.instances[0].id
    }
  }

  async function loadProviders() {
    providers.value = await invoke<Provider[]>('get_providers')
  }

  async function upsertProvider(input: object) {
    const p = await invoke<Provider>('upsert_provider', { input })
    await loadProviders()
    return p
  }

  async function deleteProvider(id: string) {
    await invoke('delete_provider', { id })
    await loadProviders()
  }

  async function switchProvider(providerId: string) {
    const inst = activeInstance()
    if (!inst) throw new Error('No instance')
    await invoke('switch_provider', { configDir: inst.config_dir, providerId })
    await loadConfig()
  }

  async function saveConfig(cfg: AppConfig) {
    await invoke('save_app_config', { cfg })
    await loadConfig()
  }

  return { providers, config, activeInstanceId, activeInstance, init, loadProviders, upsertProvider, deleteProvider, switchProvider, saveConfig }
})
