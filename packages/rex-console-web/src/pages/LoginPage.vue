<script setup lang="ts">
import { ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import Button from '@/components/ui/Button.vue'

const { t } = useI18n()
const auth = useAuthStore()
const router = useRouter()
const route = useRoute()

const password = ref('')
const errorMsg = ref('')

async function handleLogin() {
  if (!password.value) return
  errorMsg.value = ''
  try {
    await auth.login(password.value)
    const redirect = (route.query.redirect as string) || '/workspace'
    router.push(redirect)
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : t('login.loginFailed')
  }
}
</script>

<template>
  <div class="login">
    <div class="login-card">
      <div class="login-brand mono">
        REX<span class="accent">Hub</span>
      </div>
      <div class="login-subtitle">{{ t('login.subtitle') }}</div>

      <form class="login-form" @submit.prevent="handleLogin">
        <div class="field">
          <label class="field-label mono">{{ t('login.password') }}</label>
          <input
            v-model="password"
            type="password"
            class="field-input"
            placeholder="••••••••"
            autocomplete="current-password"
            autofocus
          />
        </div>
        <div v-if="errorMsg" class="error-msg">{{ errorMsg }}</div>
        <Button
          variant="primary"
          size="lg"
          :disabled="auth.loading || !password"
          style="width: 100%; margin-top: var(--space-4)"
        >
          {{ auth.loading ? t('login.signingIn') : t('login.signIn') }}
        </Button>
      </form>

      <div class="login-footer mono muted">
        Single-user · Self-hosted
      </div>
    </div>
  </div>
</template>

<style scoped>
.login {
  height: 100%;
  display: grid;
  place-items: center;
  background: var(--bg-deep);
}
.login-card {
  width: 360px;
  padding: var(--space-8);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
}
.login-brand {
  font-size: var(--text-2xl);
  font-weight: 700;
  text-align: center;
  margin-bottom: var(--space-1);
}
.accent {
  color: var(--accent);
}
.login-subtitle {
  text-align: center;
  font-size: var(--text-sm);
  color: var(--text-muted);
  margin-bottom: var(--space-6);
}
.login-form {
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
.login-footer {
  text-align: center;
  margin-top: var(--space-6);
  font-size: var(--text-xs);
}
</style>
