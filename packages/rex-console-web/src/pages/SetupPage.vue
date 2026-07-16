<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import Button from '@/components/ui/Button.vue'

const { t } = useI18n()
const auth = useAuthStore()
const router = useRouter()

const password = ref('')
const confirmPassword = ref('')
const errorMsg = ref('')

async function handleSetup() {
  errorMsg.value = ''
  if (password.value.length < 6) {
    errorMsg.value = t('setup.errorMinLength')
    return
  }
  if (password.value !== confirmPassword.value) {
    errorMsg.value = t('setup.errorMismatch')
    return
  }
  try {
    await auth.setupPassword(password.value)
    router.push('/workspace')
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : t('setup.errorFailed')
  }
}
</script>

<template>
  <div class="setup">
    <div class="setup-card">
      <div class="setup-brand mono">
        REX<span class="accent">Hub</span>
      </div>
      <div class="setup-subtitle">{{ t('setup.subtitle') }}</div>

      <form class="setup-form" @submit.prevent="handleSetup">
        <div class="field">
          <label class="field-label mono">{{ t('setup.password') }}</label>
          <input
            v-model="password"
            type="password"
            class="field-input"
            :placeholder="t('setup.passwordPlaceholder')"
            autocomplete="new-password"
            autofocus
          />
        </div>
        <div class="field">
          <label class="field-label mono">{{ t('setup.confirmPassword') }}</label>
          <input
            v-model="confirmPassword"
            type="password"
            class="field-input"
            :placeholder="t('setup.confirmPlaceholder')"
            autocomplete="new-password"
          />
        </div>
        <div v-if="errorMsg" class="error-msg">{{ errorMsg }}</div>
        <Button
          variant="primary"
          size="lg"
          :disabled="auth.loading || !password || !confirmPassword"
          style="width: 100%; margin-top: var(--space-4)"
        >
          {{ auth.loading ? t('setup.setting') : t('setup.setPassword') }}
        </Button>
      </form>

      <div class="setup-hint muted">
        {{ t('setup.hint') }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.setup {
  height: 100%;
  display: grid;
  place-items: center;
  background: var(--bg-deep);
}
.setup-card {
  width: 360px;
  padding: var(--space-8);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
}
.setup-brand {
  font-size: var(--text-2xl);
  font-weight: 700;
  text-align: center;
  margin-bottom: var(--space-1);
}
.accent {
  color: var(--accent);
}
.setup-subtitle {
  text-align: center;
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-6);
}
.setup-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.field-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.field-input {
  height: 40px;
  padding: 0 var(--space-3);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-size: var(--text-base);
  outline: none;
  transition: border-color var(--transition);
}
.field-input::placeholder {
  color: var(--text-muted);
}
.field-input:focus {
  border-color: var(--accent);
}
.error-msg {
  color: var(--danger);
  font-size: var(--text-sm);
  text-align: center;
}
.setup-hint {
  text-align: center;
  margin-top: var(--space-6);
  font-size: var(--text-xs);
}
</style>
