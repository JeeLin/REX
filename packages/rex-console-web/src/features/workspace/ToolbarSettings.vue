<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import Switch from '@/components/ui/Switch.vue'

const { t } = useI18n()

interface ToolbarConfig {
  splitH: boolean
  splitV: boolean
  fullscreen: boolean
  f1Help: boolean
  commandPalette: boolean
}

const STORAGE_KEY = 'rex-toolbar-config'

const defaultConfig: ToolbarConfig = {
  splitH: true,
  splitV: true,
  fullscreen: true,
  f1Help: true,
  commandPalette: true,
}

const config = ref<ToolbarConfig>({ ...defaultConfig })

const emit = defineEmits<{
  'update:config': [config: ToolbarConfig]
}>()

function loadConfig(): ToolbarConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<ToolbarConfig>
      return { ...defaultConfig, ...parsed }
    }
  } catch { /* ignore */ }
  return { ...defaultConfig }
}

function saveConfig() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config.value))
  emit('update:config', { ...config.value })
}

function resetToDefault() {
  config.value = { ...defaultConfig }
  saveConfig()
}

onMounted(() => {
  config.value = loadConfig()
  emit('update:config', { ...config.value })
})

watch(config, saveConfig, { deep: true })
</script>

<template>
  <div class="toolbar-settings">
    <h4 class="toolbar-settings__title">{{ t('workspace.toolbarSettings.title') }}</h4>
    <div class="toolbar-settings__items">
      <label class="toolbar-settings__item">
        <span class="toolbar-settings__label">{{ t('workspace.toolbarSettings.splitH') }}</span>
        <Switch v-model="config.splitH" size="sm" />
      </label>
      <label class="toolbar-settings__item">
        <span class="toolbar-settings__label">{{ t('workspace.toolbarSettings.splitV') }}</span>
        <Switch v-model="config.splitV" size="sm" />
      </label>
      <label class="toolbar-settings__item">
        <span class="toolbar-settings__label">{{ t('workspace.toolbarSettings.fullscreen') }}</span>
        <Switch v-model="config.fullscreen" size="sm" />
      </label>
      <label class="toolbar-settings__item">
        <span class="toolbar-settings__label">{{ t('workspace.toolbarSettings.f1Help') }}</span>
        <Switch v-model="config.f1Help" size="sm" />
      </label>
      <label class="toolbar-settings__item">
        <span class="toolbar-settings__label">{{ t('workspace.toolbarSettings.commandPalette') }}</span>
        <Switch v-model="config.commandPalette" size="sm" />
      </label>
    </div>
    <button class="toolbar-settings__reset" @click="resetToDefault">
      {{ t('workspace.toolbarSettings.resetToDefault') }}
    </button>
  </div>
</template>

<style scoped>
.toolbar-settings {
  min-width: 220px;
  padding: var(--space-3);
}

.toolbar-settings__title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--space-3);
}

.toolbar-settings__items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.toolbar-settings__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-1) 0;
  cursor: pointer;
}

.toolbar-settings__label {
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.toolbar-settings__reset {
  margin-top: var(--space-3);
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: color var(--transition), border-color var(--transition);
}

.toolbar-settings__reset:hover {
  color: var(--text-primary);
  border-color: var(--text-muted);
}
</style>
