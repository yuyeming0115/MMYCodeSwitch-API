<script setup lang="ts">
import { ref } from 'vue'
import { darkTheme } from 'naive-ui'
import AppContent from './components/AppContent.vue'

const isDark = ref(false)
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
body { font-family: Inter, system-ui, sans-serif; background: #f5f5f5; }
.app { display: flex; flex-direction: column; height: 100vh; }
.topbar { display: flex; align-items: center; gap: 12px; padding: 12px 16px; background: #fff; border-bottom: 1px solid #e8e8e8; }
.title { font-size: 16px; font-weight: 700; flex: 1; }
.active-bar { padding: 8px 16px; background: #f0faf5; border-bottom: 1px solid #d0ead8; font-size: 14px; }
.content { flex: 1; padding: 16px; overflow-y: auto; }
</style>