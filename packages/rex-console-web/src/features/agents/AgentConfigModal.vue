<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click="$emit('close')">
      <div class="modal-panel" @click.stop>
        <div class="modal-header">
          <div class="modal-title">{{ t('ctx.configTitle') }}</div>
          <button class="modal-close" @click="$emit('close')">×</button>
        </div>
        <div v-if="agent" class="modal-body">
          <div class="config-section">{{ t('ctx.basicInfo') }}</div>
          <div class="config-row">
            <span class="config-label">{{ t('ctx.environment') }}</span>
            <span class="config-value">{{ agent.environment_id }}</span>
          </div>
          <div class="config-row">
            <span class="config-label">Agent ID</span>
            <span class="config-value">{{ agent.id }}</span>
          </div>
          <div class="config-row">
            <span class="config-label">{{ t('agent.version') }}</span>
            <span class="config-value">{{ agent.version }}</span>
          </div>

          <div class="config-section">{{ t('ctx.connection') }}</div>
          <div class="config-row">
            <span class="config-label">{{ t('ctx.serverAddr') }}</span>
            <span class="config-value">—</span>
          </div>
          <div class="config-row">
            <span class="config-label">{{ t('ctx.registrationToken') }}</span>
            <span class="config-value token-value" @click="copyToken">
              {{ tokenCopied ? t('common.copySuccess') : t('ctx.clickToCopy') }}
            </span>
          </div>

          <div class="config-section">{{ t('ctx.update') }}</div>
          <div class="config-row">
            <span class="config-label">{{ t('ctx.autoUpdate') }}</span>
            <div
              class="settings-toggle"
              :class="{ active: autoUpdate }"
              @click="toggleAutoUpdate"
            ></div>
          </div>
          <div class="config-row">
            <span class="config-label">{{ t('ctx.currentVersion') }}</span>
            <span class="config-value" style="color: var(--text-muted)">{{ agent.version }}</span>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { getAgentConfig, updateAgentConfig } from '@/api/agent'
import type { Agent } from '@/api/agent'

const props = defineProps<{ agent: Agent | null; visible: boolean }>()
defineEmits<{ close: [] }>()

const { t } = useI18n()
const tokenCopied = ref(false)
const autoUpdate = ref(true)

// Load config when modal opens
watch(() => props.visible, async (v) => {
  if (v && props.agent) {
    try {
      const cfg = await getAgentConfig(props.agent.id)
      autoUpdate.value = cfg.auto_update
    } catch {
      // keep default
    }
  }
})

async function toggleAutoUpdate() {
  if (!props.agent) return
  autoUpdate.value = !autoUpdate.value
  try {
    await updateAgentConfig(props.agent.id, { auto_update: autoUpdate.value })
  } catch {
    autoUpdate.value = !autoUpdate.value // revert on error
  }
}

function copyToken() {
  navigator.clipboard?.writeText('mock-token-' + Date.now())
  tokenCopied.value = true
  setTimeout(() => { tokenCopied.value = false }, 2000)
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.5);
  backdrop-filter: blur(4px);
}

.modal-panel {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 520px;
  max-width: 90vw;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: var(--shadow-lg);
  animation: modalIn 0.2s ease;
}

@keyframes modalIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-xl);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  background: var(--bg-elevated);
  z-index: 1;
}

.modal-title {
  font-family: var(--font-mono);
  font-size: var(--fs-md);
  font-weight: 600;
}

.modal-close {
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--fs-md);
}

.modal-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.modal-body {
  padding: var(--sp-xl);
}

.config-section {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
  margin-top: var(--sp-lg);
  margin-bottom: var(--sp-sm);
}

.config-section:first-child {
  margin-top: 0;
}

.config-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-md) 0;
  border-bottom: 1px solid var(--border);
}

.config-row:last-child {
  border-bottom: none;
}

.config-label {
  font-size: var(--fs-sm);
  color: var(--text-secondary);
}

.config-value {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--text-primary);
  text-align: right;
}

.token-value {
  padding: var(--sp-xs) var(--sp-sm);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: border-color var(--transition-fast);
}

.token-value:hover {
  border-color: var(--accent);
}

.settings-toggle {
  width: 40px;
  height: 22px;
  background: var(--border);
  border-radius: 11px;
  position: relative;
  cursor: pointer;
  transition: background var(--transition-fast);
  flex-shrink: 0;
}

.settings-toggle.active {
  background: var(--accent);
}

.settings-toggle::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  transition: transform var(--transition-fast);
}

.settings-toggle.active::after {
  transform: translateX(18px);
}
</style>
