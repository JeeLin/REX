<template>
  <div class="update-section">
    <h3>{{ t('settings.update.title') }}</h3>
    <div class="version-info">
      <span>{{ t('settings.update.currentVersion') }}: {{ status?.current_version || '...' }}</span>
      <span v-if="status?.git_commit" class="commit">({{ status.git_commit.slice(0, 7) }})</span>
    </div>
    <div v-if="status?.last_checked" class="last-checked">
      {{ t('settings.update.lastChecked') }}: {{ formatTime(status.last_checked) }}
    </div>
    <button class="btn btn-primary" :disabled="checking || downloading" @click="handleCheck">
      {{ checking ? t('settings.update.checking') : t('settings.update.checkNow') }}
    </button>
    <div v-if="status?.update_available" class="update-available">
      ⚠ {{ t('settings.update.foundNew') }} {{ status.latest_version }}
      <button
        class="btn btn-download"
        :disabled="downloading || applying"
        @click="handleDownload"
      >
        {{ downloading ? t('settings.update.downloading') + ' ' + downloadPercent + '%' : t('settings.update.download') }}
      </button>
    </div>
    <div v-if="downloadReady" class="download-ready">
      ✓ {{ t('settings.update.ready') }}
      <button class="btn btn-apply" :disabled="applying" @click="handleApply">
        {{ applying ? t('settings.update.updating') : t('settings.update.applyNow') }}
      </button>
    </div>
    <div v-if="downloadError" class="download-error">
      ✗ {{ t('settings.update.downloadFailed') }}: {{ downloadError }}
    </div>
    <div v-else-if="checked && !checking && !status?.update_available" class="up-to-date">
      ✓ {{ t('settings.update.upToDate') }}
    </div>

    <!-- 版本总览（可折叠） -->
    <div v-if="agentVersions.length" class="agent-versions">
      <button class="version-overview-toggle" @click="toggleVersionOverview">
        <span class="toggle-icon">{{ versionOverviewExpanded ? '▾' : '▸' }}</span>
        {{ t('settings.update.versionOverview') }}
      </button>
      <div v-if="versionOverviewExpanded" class="version-overview-content">
        <!-- Hub 行 -->
        <div class="version-row hub-row">
          <span class="version-icon">⊞</span>
          <span class="version-name">{{ t('settings.update.hubLabel') }}</span>
          <span class="version-platform" />
          <span class="version-ver">{{ status?.current_version || '...' }}</span>
          <span class="version-tag latest">{{ t('settings.update.hubUpToDate') }}</span>
        </div>
        <!-- Agent 行 -->
        <div v-for="av in agentVersions" :key="av.agent_id" class="version-row agent-row">
          <span class="version-icon">⬡</span>
          <span class="version-name">{{ av.name }}</span>
          <span class="version-platform">{{ av.platform }}</span>
          <template v-if="av.status === 'online'">
            <span class="version-ver">{{ av.version }}</span>
            <span class="version-tag" :class="av.needs_update ? 'outdated' : 'latest'">
              {{ agentStatusLabel(av) }}
            </span>
          </template>
          <span v-else class="version-offline">{{ t('settings.update.offline') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getUpdateStatus, checkUpdate, downloadUpdate, applyUpdate, listAgentVersions } from '@/api/update'
import type { UpdateStatusResponse, AgentVersionInfo } from '@/api/update'

const { t } = useI18n()
const versionOverviewExpanded = ref(
  localStorage.getItem('rex-version-overview-expanded') === 'true'
)
const status = ref<UpdateStatusResponse | null>(null)
const checking = ref(false)
const checked = ref(false)
const downloading = ref(false)
const downloadPercent = ref(0)
const downloadReady = ref(false)
const downloadError = ref('')
const applying = ref(false)
const agentVersions = ref<AgentVersionInfo[]>([])

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

async function handleCheck() {
  checking.value = true
  try {
    status.value = await checkUpdate()
    checked.value = true
  } catch {
    // ignore
  } finally {
    checking.value = false
  }
}

async function handleDownload() {
  if (!status.value?.latest_version) return
  downloading.value = true
  downloadError.value = ''
  downloadReady.value = false
  downloadPercent.value = 0
  try {
    const result = await downloadUpdate(status.value.latest_version)
    if (result.status === 'ready') {
      downloadReady.value = true
    } else {
      downloadError.value = result.message
    }
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e)
    downloadError.value = msg
  } finally {
    downloading.value = false
  }
}

async function handleApply() {
  applying.value = true
  try {
    await applyUpdate()
    // 应用更新后会触发 supervisor 重启，页面会刷新
  } catch {
    applying.value = false
  }
}

onMounted(async () => {
  try {
    status.value = await getUpdateStatus()
  } catch {
    // ignore
  }
  try {
    agentVersions.value = await listAgentVersions()
  } catch {
    // ignore
  }
})

function toggleVersionOverview() {
  versionOverviewExpanded.value = !versionOverviewExpanded.value
  localStorage.setItem('rex-version-overview-expanded', String(versionOverviewExpanded.value))
}

function agentStatusLabel(av: AgentVersionInfo): string {
  if (av.status !== 'online') return t('settings.update.offline')
  if (av.needs_update) return t('settings.update.hasNewVersion')
  return t('settings.update.upToDate')
}
</script>

<style scoped>
.update-section {
  padding: var(--sp-lg);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}

.update-section h3 {
  margin: 0 0 var(--sp-md);
  font-size: var(--fs-lg);
  color: var(--text-primary);
}

.version-info {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  margin-bottom: var(--sp-sm);
}

.version-info .commit {
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.last-checked {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  margin-bottom: var(--sp-md);
}

.update-available {
  margin-top: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  background: var(--warning-bg, rgba(255, 193, 7, 0.1));
  border: 1px solid var(--warning, #ffc107);
  border-radius: var(--radius-sm);
  color: var(--warning, #ffc107);
  font-size: var(--fs-sm);
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.download-ready {
  margin-top: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  background: var(--success-bg, rgba(40, 167, 69, 0.1));
  border: 1px solid var(--success, #28a745);
  border-radius: var(--radius-sm);
  color: var(--success, #28a745);
  font-size: var(--fs-sm);
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.download-error {
  margin-top: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  background: rgba(220, 53, 69, 0.1);
  border: 1px solid #dc3545;
  border-radius: var(--radius-sm);
  color: #dc3545;
  font-size: var(--fs-sm);
}

.up-to-date {
  margin-top: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  background: var(--success-bg, rgba(40, 167, 69, 0.1));
  border: 1px solid var(--success, #28a745);
  border-radius: var(--radius-sm);
  color: var(--success, #28a745);
  font-size: var(--fs-sm);
}

.btn-download {
  background: var(--info, #17a2b8);
  color: white;
  border: none;
  padding: 4px 12px;
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  cursor: pointer;
}

.btn-download:hover {
  opacity: 0.9;
}

.btn-apply {
  background: var(--success, #28a745);
  color: white;
  border: none;
  padding: 4px 12px;
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  cursor: pointer;
}

.btn-apply:hover {
  opacity: 0.9;
}

/* Agent 版本总览 */
.agent-versions {
  margin-top: var(--sp-lg);
  padding-top: var(--sp-md);
  border-top: 1px solid var(--border);
}

.version-overview-toggle {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  cursor: pointer;
  padding: var(--sp-xs) 0;
}

.version-overview-toggle:hover {
  color: var(--text-primary);
}

.toggle-icon {
  font-size: var(--fs-xs);
  width: 1em;
}

.version-overview-content {
  margin-top: var(--sp-sm);
}

.version-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) 0;
  font-size: var(--fs-sm);
}

.version-icon {
  color: var(--text-muted);
  width: 1.2em;
  text-align: center;
  flex-shrink: 0;
}

.version-name {
  color: var(--text-primary);
  min-width: 100px;
}

.version-platform {
  color: var(--text-muted);
  font-size: var(--fs-xs);
  min-width: 120px;
}

.version-ver {
  font-family: var(--font-mono);
  color: var(--text-muted);
}

.version-tag {
  margin-left: auto;
  font-size: var(--fs-xs);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
}

.version-tag.latest {
  color: var(--success, #28a745);
}

.version-tag.outdated {
  color: var(--warning, #ffc107);
}

.version-offline {
  color: var(--text-muted);
  font-size: var(--fs-xs);
  margin-left: auto;
}
</style>
