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
body { font-family: Inter, system-ui, sans-serif; background: #f5f5f5; color: #333; }
body.dark { background: #1a1a1a; color: #eee; }
.app { display: flex; flex-direction: column; height: 100vh; }
</style>