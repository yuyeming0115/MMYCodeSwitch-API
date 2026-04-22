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
  defaultConfigDir?: string
  activeProjects: ActiveProject[]
  backupExportPath?: string
}

export interface ActiveProject {
  id: string
  name: string
  project_path: string
  provider_id: string
  provider_name: string
  created_at: string
  updated_at: string
  /** 项目专属配置目录路径（新增） */
  config_dir?: string
}

export interface SessionArchive {
  id: string
  provider_id: string
  provider_name: string
  switched_at: string
  config_snapshot: Record<string, string>
}

export const useAppStore = defineStore('app', () => {
  const providers = ref<Provider[]>([])
  const config = ref<AppConfig>({ language: 'zh', instances: [], activeProjects: [] })
  const activeInstanceId = ref('default')
  const activeProjects = ref<ActiveProject[]>([])

  const activeInstance = () => config.value.instances.find(i => i.id === activeInstanceId.value) ?? config.value.instances[0]

  async function init() {
    await invoke('init_app')
    await loadConfig()
    await loadProviders()
    await loadActiveProjects()
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

  async function loadActiveProjects() {
    activeProjects.value = await invoke<ActiveProject[]>('get_active_projects')
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

  /// 多项目模式：注入 API 到指定项目文件夹（方案C：项目专属目录）
  async function injectToProject(projectPath: string, providerId: string) {
    const result = await invoke<{
      project: ActiveProject
      was_existing: boolean
      config_dir: string
    }>('inject_to_project', { projectPath, providerId })
    await loadActiveProjects()
    return result
  }

  /// 获取项目的会话归档列表
  async function getProjectSessions(projectPath: string) {
    const sessions = await invoke<SessionArchive[]>('get_project_sessions', { projectPath })
    return sessions
  }

  /// 从已激活列表中移除项目（仅删除记录）
  async function removeActiveProject(id: string) {
    await invoke('remove_active_project', { id })
    await loadActiveProjects()
  }

  return {
    providers, config, activeInstanceId, activeInstance,
    activeProjects,
    init, loadProviders, upsertProvider, deleteProvider,
    switchProvider, saveConfig,
    injectToProject, removeActiveProject, loadActiveProjects,
    getProjectSessions,
  }
})
