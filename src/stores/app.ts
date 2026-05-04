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
  order?: number
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
  config_dir?: string
  order?: number
}

export interface SessionArchive {
  id: string
  provider_id: string
  provider_name: string
  switched_at: string
  config_snapshot: Record<string, string>
}

// Template 类型
export interface Template {
  name: string
  content: string
  created_at: string
  updated_at: string
  builtin?: boolean
}

export interface TemplateBinding {
  project_path: string
  template_name: string
  updated_at: string
}

// Skill 类型
export interface Skill {
  name: string
  content: string
  created_at: string
  updated_at: string
}

// 供应商模板类型
export interface ProviderTemplateUrl {
  label: string
  value: string
  hint?: string
  protocolHint?: string
}

export interface ProviderTemplate {
  id: string
  name: string
  icon?: string
  color?: string
  description?: string
  builtinIcon?: string
  iconFallback?: string
  baseUrls: ProviderTemplateUrl[]
  models: string[]
  keyPlaceholder?: string
  helpUrl?: string
  badge?: string
  builtin: boolean
  created_at: string
  updated_at: string
}

export const useAppStore = defineStore('app', () => {
  const providers = ref<Provider[]>([])
  const config = ref<AppConfig>({ language: 'zh', instances: [], activeProjects: [] })
  const activeInstanceId = ref('default')
  const activeProjects = ref<ActiveProject[]>([])
  const templates = ref<Template[]>([])
  const templateBindings = ref<TemplateBinding[]>([])
  const skills = ref<Skill[]>([])
  const providerTemplates = ref<ProviderTemplate[]>([])

  const activeInstance = () => config.value.instances.find(i => i.id === activeInstanceId.value) ?? config.value.instances[0]

  async function init() {
    await invoke('init_app')
    await loadConfig()
    await loadProviders()
    await loadActiveProjects()
    await loadTemplates()
    await loadTemplateBindings()
    await loadSkills()
    await loadProviderTemplates()
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

  async function reorderProviders(orderedIds: string[]) {
    await invoke('reorder_providers', { orderedIds })
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

  async function injectToProject(projectPath: string, providerId: string) {
    const result = await invoke<{
      project: ActiveProject
      was_existing: boolean
      config_dir: string
    }>('inject_to_project', { projectPath, providerId })
    await loadActiveProjects()
    return result
  }

  async function getProjectSessions(projectPath: string) {
    const sessions = await invoke<SessionArchive[]>('get_project_sessions', { projectPath })
    return sessions
  }

  async function removeActiveProject(id: string) {
    await invoke('remove_active_project', { id })
    await loadActiveProjects()
  }

  async function reorderActiveProjects(orderedIds: string[]) {
    await invoke('reorder_projects', { orderedIds })
    await loadActiveProjects()
  }

  // Template 管理
  async function loadTemplates() {
    templates.value = await invoke<Template[]>('get_templates')
  }

  async function saveTemplate(name: string, content: string) {
    await invoke('save_template', { name, content })
    await loadTemplates()
  }

  async function deleteTemplate(name: string) {
    await invoke('delete_template', { name })
    await loadTemplates()
  }

  async function adoptTemplate(name: string) {
    await invoke('adopt_template', { name })
    await loadTemplates()
  }

  async function loadTemplateBindings() {
    templateBindings.value = await invoke<TemplateBinding[]>('get_template_bindings')
  }

  async function bindTemplate(projectPath: string, templateName: string) {
    await invoke('bind_template', { projectPath, templateName })
    await loadTemplateBindings()
  }

  async function unbindTemplate(projectPath: string) {
    await invoke('unbind_template', { projectPath })
    await loadTemplateBindings()
  }

  async function injectClaudeMd(projectPath: string, templateName: string) {
    const path = await invoke<string>('inject_claude_md', { projectPath, templateName })
    return path
  }

  async function getProjectTemplate(projectPath: string) {
    const name = await invoke<string | null>('get_project_template', { projectPath })
    return name
  }

  // Skill 管理
  async function loadSkills() {
    skills.value = await invoke<Skill[]>('get_skills')
  }

  async function saveSkill(name: string, content: string) {
    await invoke('save_skill', { name, content })
    await loadSkills()
  }

  async function deleteSkill(name: string) {
    await invoke('delete_skill', { name })
    await loadSkills()
  }

  // Provider Templates 管理
  async function loadProviderTemplates() {
    providerTemplates.value = await invoke<ProviderTemplate[]>('get_provider_templates')
  }

  async function saveProviderTemplate(input: object) {
    await invoke('save_provider_template', { input })
    await loadProviderTemplates()
  }

  async function deleteProviderTemplate(id: string) {
    await invoke('delete_provider_template', { id })
    await loadProviderTemplates()
  }

  return {
    providers, config, activeInstanceId, activeInstance,
    activeProjects, templates, templateBindings, skills, providerTemplates,
    init, loadProviders, upsertProvider, deleteProvider, reorderProviders, reorderActiveProjects,
    switchProvider, saveConfig,
    injectToProject, removeActiveProject, loadActiveProjects,
    getProjectSessions,
    // Template
    loadTemplates, saveTemplate, deleteTemplate, adoptTemplate,
    loadTemplateBindings, bindTemplate, unbindTemplate,
    injectClaudeMd, getProjectTemplate,
    // Skill
    loadSkills, saveSkill, deleteSkill,
    // Provider Templates
    loadProviderTemplates, saveProviderTemplate, deleteProviderTemplate,
  }
})