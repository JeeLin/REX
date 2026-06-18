<template>
  <div class="env-new-page">
    <div class="env-new-header">
      <router-link to="/environments" class="btn btn-ghost btn-sm">
        ← {{ t('common.back') }}
      </router-link>
      <h2 class="page-title">{{ t('env.createTitle') }}</h2>
    </div>

    <form class="env-form card" @submit.prevent="handleCreate">
      <div class="form-group">
        <label class="form-label">{{ t('env.name') }}</label>
        <input
          v-model="form.name"
          class="form-input"
          :placeholder="t('env.namePlaceholder')"
          required
        />
      </div>

      <div class="form-group">
        <label class="form-label">{{ t('env.description') }}</label>
        <textarea
          v-model="form.description"
          class="form-input"
          :placeholder="t('env.descriptionPlaceholder')"
          rows="3"
        />
      </div>

      <div class="form-group">
        <label class="form-label">{{ t('env.connectionMode') }}</label>
        <div class="mode-cards">
          <div
            class="mode-card"
            :class="{ active: form.connection_mode === 'agent_proxy' }"
            @click="form.connection_mode = 'agent_proxy'"
          >
            <div class="mode-icon">⬡</div>
            <div class="mode-title">{{ t('env.agentProxy') }}</div>
            <div class="mode-desc">{{ t('env.agentProxyDesc') }}</div>
          </div>
          <div
            class="mode-card"
            :class="{ active: form.connection_mode === 'direct' }"
            @click="form.connection_mode = 'direct'"
          >
            <div class="mode-icon">→</div>
            <div class="mode-title">{{ t('env.direct') }}</div>
            <div class="mode-desc">{{ t('env.directDesc') }}</div>
          </div>
        </div>
      </div>

      <div v-if="errorMsg" class="login-error">{{ errorMsg }}</div>

      <div class="form-actions">
        <router-link to="/environments" class="btn btn-ghost">{{ t('common.cancel') }}</router-link>
        <button type="submit" class="btn btn-primary" :disabled="loading || !form.name.trim()">
          {{ loading ? t('common.loading') : t('env.createBtn') }}
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import client from '@/api/client'

const { t } = useI18n()
const router = useRouter()

const form = reactive({
  name: '',
  description: '',
  connection_mode: 'direct',
})

const loading = ref(false)
const errorMsg = ref('')

async function handleCreate() {
  if (!form.name.trim()) return
  loading.value = true
  errorMsg.value = ''
  try {
    await client.post('/environments', form)
    router.push('/environments')
  } catch {
    errorMsg.value = t('common.error')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.env-new-header {
  display: flex;
  align-items: center;
  gap: var(--sp-lg);
  margin-bottom: var(--sp-xl);
}

.env-form {
  max-width: 560px;
  display: flex;
  flex-direction: column;
  gap: var(--sp-xl);
}

.mode-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--sp-md);
}

.mode-card {
  background: var(--bg-deep);
  border: 2px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  cursor: pointer;
  transition: all var(--transition-base);
  text-align: center;
}

.mode-card:hover {
  border-color: var(--accent);
}

.mode-card.active {
  border-color: var(--accent);
  background: rgba(232, 145, 45, 0.05);
  box-shadow: 0 0 20px rgba(232, 145, 45, 0.08);
}

.mode-icon {
  font-size: 24px;
  margin-bottom: var(--sp-sm);
}

.mode-title {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-base);
  margin-bottom: var(--sp-xs);
}

.mode-desc {
  font-size: var(--fs-xs);
  color: var(--text-secondary);
  line-height: var(--lh-relaxed);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}

.login-error {
  color: var(--danger);
  font-size: var(--fs-sm);
}
</style>
