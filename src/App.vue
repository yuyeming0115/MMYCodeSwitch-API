<script setup lang="ts">
import { ref, watchEffect } from 'vue'
import { darkTheme } from 'naive-ui'
import AppContent from './components/AppContent.vue'

const isDark = ref(false)
watchEffect(() => {
  document.body.classList.toggle('dark', isDark.value)
})
</script>

<template>
  <n-config-provider :theme="isDark ? darkTheme : null">
    <n-message-provider>
      <n-dialog-provider>
        <AppContent v-model:is-dark="isDark" />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }

/* ==================== 全局滚动条（全局样式，不受 Vue scoped 影响） ==================== */

/* 默认模式 */
html {
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.2) transparent;
}
html::-webkit-scrollbar { width: 6px; }
html::-webkit-scrollbar-track { background: transparent; }
html::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.2);
  border-radius: 10px;
}
html::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.4);
}

/* 主内容区 .content（AppContent.vue） */
.content {
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.25) transparent;
}
.content::-webkit-scrollbar { width: 5px; }
.content::-webkit-scrollbar-track { background: transparent; }
.content::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.25);
  border-radius: 10px;
}
.content::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.45);
}

/* 项目列表 .project-cards（ProjectList.vue） */
.project-cards {
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.25) transparent;
}
.project-cards::-webkit-scrollbar { width: 5px; }
.project-cards::-webkit-scrollbar-track { background: transparent; }
.project-cards::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.25);
  border-radius: 10px;
}
.project-cards::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.45);
}

/* 页面内容 .page-content（ProviderForm/Settings/TemplateManager/SkillManager） */
.page-content {
  scrollbar-width: thin;
  scrollbar-color: rgba(128,128,128,0.2) transparent;
}
.page-content::-webkit-scrollbar { width: 6px; }
.page-content::-webkit-scrollbar-track { background: transparent; }
.page-content::-webkit-scrollbar-thumb {
  background: rgba(128,128,128,0.2);
  border-radius: 10px;
}
.page-content::-webkit-scrollbar-thumb:hover {
  background: rgba(128,128,128,0.4);
}

/* ========== 深色模式 ========== */

body.dark { background: #1a1a1a; color: #eee; }

/* 全局深色滚动条 */
body.dark {
  scrollbar-width: thin;
  scrollbar-color: rgba(200,200,200,0.35) #2a2a2a !important;
}
body.dark ::-webkit-scrollbar { background: #2a2a2a; }
body.dark ::-webkit-scrollbar-track { background: #2a2a2a; }
body.dark ::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.35); border-radius: 10px; }
body.dark ::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.5); }

/* html 根元素深色滚动条 */
body.dark html { scrollbar-color: rgba(200,200,200,0.35) #2a2a2a !important; }
body.dark html::-webkit-scrollbar { background: #2a2a2a; }
body.dark html::-webkit-scrollbar-track { background: #2a2a2a; }
body.dark html::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.35) !important; }
body.dark html::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.5) !important; }

/* 主内容区 .content 深色滚动条 */
body.dark .content { scrollbar-color: rgba(200,200,200,0.35) #2a2a2a !important; }
body.dark .content::-webkit-scrollbar { background: #2a2a2a; }
body.dark .content::-webkit-scrollbar-track { background: #2a2a2a; }
body.dark .content::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.35) !important; }
body.dark .content::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.5) !important; }

/* 项目列表 .project-cards 深色滚动条 */
body.dark .project-cards { scrollbar-color: rgba(200,200,200,0.35) #2a2a2a !important; }
body.dark .project-cards::-webkit-scrollbar { background: #2a2a2a; }
body.dark .project-cards::-webkit-scrollbar-track { background: #2a2a2a; }
body.dark .project-cards::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.35) !important; }
body.dark .project-cards::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.5) !important; }

/* 页面内容 .page-content 深色滚动条 */
body.dark .page-content { scrollbar-color: rgba(200,200,200,0.35) #2a2a2a !important; }
body.dark .page-content::-webkit-scrollbar { background: #2a2a2a; }
body.dark .page-content::-webkit-scrollbar-track { background: #2a2a2a; }
body.dark .page-content::-webkit-scrollbar-thumb { background: rgba(200,200,200,0.35) !important; }
body.dark .page-content::-webkit-scrollbar-thumb:hover { background: rgba(200,200,200,0.5) !important; }

/* ==================== 全局布局 ==================== */

body { font-family: Inter, system-ui, sans-serif; background: #f5f5f5; color: #333; }
.app { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }

/* 自定义标题栏 - 支持 Tauri 拖拽 */
.titlebar {
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
  background: #fff;
  border-bottom: 1px solid #eee;
  flex-shrink: 0;
}
body.dark .titlebar {
  background: #242424;
  border-bottom-color: #333;
}
.titlebar-left {
  display: flex;
  align-items: center;
}
.titlebar-icon {
  width: 20px;
  height: 20px;
  margin-right: 8px;
}
.titlebar-title {
  font-size: 13px;
  font-weight: 600;
  color: #555;
}
body.dark .titlebar-title { color: #ccc; }
.titlebar-page-title {
  font-size: 13px;
  font-weight: 600;
  color: #555;
}
body.dark .titlebar-page-title { color: #ccc; }
.titlebar-controls {
  display: flex;
  gap: 0;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}
.titlebar-btn {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s;
  background: transparent;
  padding: 0;
  border: none;
}
.titlebar-btn:hover { background: rgba(215, 119, 87, 0.12); }
.titlebar-btn:hover svg { color: #d77757; }
body.dark .titlebar-btn:hover { background: rgba(215, 119, 87, 0.15); }
body.dark .titlebar-btn:hover svg { color: #d77757; }
.titlebar-btn.close-btn:hover { background: #e81123 !important; }
.titlebar-btn.close-btn:hover svg { color: #fff !important; }
</style>
