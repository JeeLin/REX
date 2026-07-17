<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { settingsApi, type Settings } from '@/api/settings'
import { updateApi, type VersionInfo } from '@/api/agents'
import Card from '@/components/ui/Card.vue'
import Badge from '@/components/ui/Badge.vue'

const { t } = useI18n()
const settings = ref<Settings>({
  theme: 'dark',
  language: 'zh',
  terminal_font: 'JetBrains Mono',
  terminal_font_size: '14',
})
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')

// 更新相关
const versionInfo = ref<VersionInfo | null>(null)

onMounted(async () => {
  try {
    settings.value = await settingsApi.get()
    document.documentElement.dataset.theme = settings.value.theme === 'light' ? 'light' : undefined
    localStorage.setItem('rex-theme', settings.value.theme)
  } catch {
    // ignore
  } finally {
    loading.value = false
  }

  // 加载版本信息
  try {
    versionInfo.value = await updateApi.getVersion()
  } catch {
    // ignore
  }
})

async function saveSettings() {
  saving.value = true
  saveMessage.value = ''
  try {
    await settingsApi.update(settings.value)
    document.documentElement.dataset.theme = settings.value.theme === 'light' ? 'light' : undefined
    localStorage.setItem('rex-theme', settings.value.theme)
    saveMessage.value = t('settings.saved')
    setTimeout(() => saveMessage.value = '', 2000)
  } catch (e: unknown) {
    saveMessage.value = e instanceof Error ? e.message : t('settings.saveFailed')
  } finally {
    saving.value = false
  }
}


</script>

<template>
  <div class="settings-page">
    <h1 class="page-title">{{ t('settings.title') }}</h1>

    <Card class="settings-section">
      <h2 class="section-title">{{ t('settings.appearance') }}</h2>
      <div class="form-group">
        <label class="form-label">{{ t('settings.theme') }}</label>
        <select v-model="settings.theme" class="form-input">
          <option value="dark">{{ t('settings.dark') }}</option>
          <option value="light">{{ t('settings.light') }}</option>
        </select>
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.language') }}</label>
        <select v-model="settings.language" class="form-input">
          <option value="zh">中文</option>
          <option value="en">English</option>
        </select>
      </div>
    </Card>

    <Card class="settings-section">
      <h2 class="section-title">{{ t('settings.terminal') }}</h2>
      <div class="form-group">
        <label class="form-label">{{ t('settings.fontFamily') }}</label>
        <input v-model="settings.terminal_font" type="text" class="form-input" />
      </div>
      <div class="form-group">
        <label class="form-label">{{ t('settings.fontSize') }}</label>
        <input v-model="settings.terminal_font_size" type="number" class="form-input" min="10" max="24" />
      </div>
    </Card>

    <Card class="settings-section">
      <h2 class="section-title">{{ t('settings.updates') }}</h2>
      <div class="version-row">
        <span class="muted">{{ t('settings.hubVersion') }}</span>
        <span class="mono">{{ versionInfo?.hub_version || '—' }}</span>
      </div>

      <!-- Agent 版本总览 -->
      <div v-if="versionInfo?.agents?.length" class="agent-versions">
        <h3 class="sub-title">{{ t('settings.agentVersions') }}</h3>
        <div class="version-table">
          <div class="version-row" v-for="a in versionInfo.agents" :key="a.agent_id">
            <span class="agent-name-cell">
              <span class="status-dot" :class="a.is_online ? 'online' : 'offline'"></span>
              {{ a.name }}
            </span>
            <span class="mono">{{ a.version || '—' }}</span>
            <Badge :variant="a.is_up_to_date ? 'success' : a.is_online ? 'warning' : 'default'" size="sm">
              {{ a.is_up_to_date ? '✓' : a.is_online ? t('settings.canUpdate') : t('agents.offline') }}
            </Badge>
          </div>
        </div>
      </div>
    </Card>

    <div class="save-bar">
      <span v-if="saveMessage" class="save-message" :class="{ error: saveMessage.includes('failed') }">{{ saveMessage }}</span>
      <Button variant="primary" :loading="saving" @click="saveSettings">{{ t('settings.saveSettings') }}</Button>
    </div>
  </div>
</template>

<style scoped>
.settings-page { max-width: 600px; }
.page-title { font-size: var(--text-xl); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-6); }
.settings-section { margin-bottom: var(--space-4); }
.section-title { font-size: var(--text-md); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-4); }
.form-group { margin-bottom: var(--space-3); }
.form-label { display: block; font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-1); }
.form-input {
  width: 100%; background: var(--bg-deep); border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 12px; color: var(--text-primary); font-size: var(--text-sm); outline: none;
}
.form-input:focus { border-color: var(--accent); }
.save-bar { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-3); margin-top: var(--space-4); }
.save-message { font-size: var(--text-sm); color: var(--success); }
.save-message.error { color: var(--danger); }
.version-row { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-2); font-size: var(--text-sm); }
.muted { color: var(--text-muted); }
.mono { font-family: var(--font-mono); }
.check-result { margin-top: var(--space-3); }
.release-notes { font-size: var(--text-sm); color: var(--text-secondary); margin-top: var(--space-2); white-space: pre-wrap; max-height: 200px; overflow-y: auto; }
.agent-versions { margin-top: var(--space-4); border-top: 1px solid var(--border); padding-top: var(--space-3); }
.sub-title { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); margin-bottom: var(--space-2); }
.version-table { display: flex; flex-direction: column; gap: var(--space-1); }
.agent-name-cell { display: flex; align-items: center; gap: var(--space-2); min-width: 120px; }
.status-dot { width: 6px; height: 6px; border-radius: 50%; }
.status-dot.online { background: var(--success); }
.status-dot.offline { background: var(--text-muted); }
</style>
