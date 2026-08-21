<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/api/client'
import type { LoginResponse } from '@/types/auth'

const emit = defineEmits<{ cancel: [] }>()

const { t } = useI18n()
const auth = useAuthStore()
const password = ref('')
const loading = ref(false)
const errorMsg = ref('')

async function handleRefresh() {
  if (!password.value.trim()) return
  loading.value = true
  errorMsg.value = ''
  try {
    const res = await api.post<LoginResponse>('/auth/login', { password: password.value })
    auth.setToken(res.token)
    password.value = ''
    emit('cancel')
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

function handleCancel() {
  auth.logout()
  window.location.href = '/login'
}
</script>

<template>
  <Teleport to="body">
    <div class="token-refresh-overlay" @click.self="handleCancel">
      <div class="token-refresh-card">
        <div class="card-header">
          <h3>{{ t('auth.tokenRefresh.title') }}</h3>
          <p class="card-desc">{{ t('auth.tokenRefresh.description') }}</p>
        </div>

        <form @submit.prevent="handleRefresh">
          <div class="field">
            <input
              v-model="password"
              type="password"
              :placeholder="t('auth.tokenRefresh.passwordPlaceholder')"
              autofocus
              :disabled="loading"
            />
          </div>

          <div v-if="errorMsg" class="error-msg">{{ errorMsg }}</div>

          <div class="actions">
            <button type="button" class="btn-secondary" :disabled="loading" @click="handleCancel">
              {{ t('auth.tokenRefresh.cancel') }}
            </button>
            <button type="submit" class="btn-primary" :disabled="loading || !password.trim()">
              {{ loading ? t('auth.tokenRefresh.refreshing') : t('auth.tokenRefresh.submit') }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.token-refresh-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
}

.token-refresh-card {
  width: 360px;
  padding: 24px;
  border-radius: 12px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.card-header h3 {
  margin: 0 0 4px;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.card-desc {
  margin: 0 0 16px;
  font-size: 13px;
  color: var(--text-secondary);
}

.field {
  margin-bottom: 12px;
}

.field input {
  width: 100%;
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
  box-sizing: border-box;
}

.field input:focus {
  border-color: var(--accent);
}

.error-msg {
  margin-bottom: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 13px;
}

.actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.btn-secondary,
.btn-primary {
  padding: 6px 16px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  border: none;
}

.btn-secondary {
  background: var(--bg-elevated);
  color: var(--text-secondary);
}

.btn-primary {
  background: var(--accent);
  color: var(--text-on-accent);
}

.btn-primary:disabled,
.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
