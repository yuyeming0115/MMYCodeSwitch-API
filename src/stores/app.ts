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
  config_dir?: string
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

export const useAppStore = defineStore('app', () => {
  const providers = ref<Provider[]>([])
  const config = ref<AppConfig>({ language: 'zh', instances: [], activeProjects: [] })
  const activeInstanceId = ref('default')
  const activeProjects = ref<ActiveProject[]>([])
  const templates = ref<Template[]>([])
  const templateBindings = ref<TemplateBinding[]>([])
  const skills = ref<Skill[]>([])

  const activeInstance = () => config.value.instances.find(i => i.id === activeInstanceId.value) ?? config.value.instances[0]

  async function init() {
    await invoke('init_app')
    await loadConfig()
    await loadProviders()
    await loadActiveProjects()
    await loadTemplates()
    await loadTemplateBindings()
    await loadSkills()
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

  return {
    providers, config, activeInstanceId, activeInstance,
    activeProjects, templates, templateBindings, skills,
    init, loadProviders, upsertProvider, deleteProvider,
    switchProvider, saveConfig,
    injectToProject, removeActiveProject, loadActiveProjects,
    getProjectSessions,
    // Template
    loadTemplates, saveTemplate, deleteTemplate,
    loadTemplateBindings, bindTemplate, unbindTemplate,
    injectClaudeMd, getProjectTemplate,
    // Skill
    loadSkills, saveSkill, deleteSkill,
  }
})