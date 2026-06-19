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
    <button class="btn btn-primary" @click="handleCheck" :disabled="checking">
      {{ checking ? t('settings.update.checking') : t('settings.update.checkNow') }}
    </button>
    <div v-if="status?.update_available" class="update-available">
      ⚠ {{ t('settings.update.foundNew') }} {{ status.latest_version }}
    </div>
    <div v-else-if="checked && !checking" class="up-to-date">
      ✓ {{ t('settings.update.upToDate') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getUpdateStatus, checkUpdate } from '@/api/update'
import type { UpdateStatusResponse } from '@/api/update'

const { t } = useI18n()
const status = ref<UpdateStatusResponse | null>(null)
const checking = ref(false)
const checked = ref(false)

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

onMounted(async () => {
  try {
    status.value = await getUpdateStatus()
  } catch {
    // ignore
  }
})
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
</style>
