<template>
  <div class="login-page">
    <div class="login-container">
      <div class="login-brand">
        <div class="login-logo">R</div>
        <h1 class="login-title">REX Hub</h1>
        <p class="login-subtitle">{{ t('auth.subtitle') }}</p>
      </div>

      <div class="login-card">
        <form class="login-form" @submit.prevent="handleLogin">
          <div class="form-group">
            <label class="form-label" for="password">{{ t('auth.password') }}</label>
            <input
              id="password"
              v-model="password"
              class="form-input"
              type="password"
              :placeholder="t('auth.password')"
              autocomplete="current-password"
              required
            />
          </div>

          <div class="remember-row">
            <label class="checkbox-label">
              <input v-model="rememberMe" type="checkbox" />
              {{ t('auth.rememberMe') }}
            </label>
          </div>

          <div v-if="errorMsg" class="login-error">{{ errorMsg }}</div>

          <button type="submit" class="btn btn-primary login-btn" :disabled="loading">
            {{ loading ? t('common.loading') : t('auth.loginBtn') }}
          </button>
        </form>
      </div>

      <div class="login-footer">
        <span class="version">{{ t('auth.version') }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'

const { t } = useI18n()
const router = useRouter()
const authStore = useAuthStore()

const username = ref('')
const password = ref('')
const rememberMe = ref(true)
const loading = ref(false)
const errorMsg = ref('')

async function handleLogin() {
  loading.value = true
  errorMsg.value = ''
  try {
    await authStore.login(password.value)
    router.push('/')
  } catch {
    errorMsg.value = t('auth.loginFailed')
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--sp-xl);
  background: var(--bg-deep);
  position: relative;
  overflow: hidden;
}

/* CRT scanline effect */
.login-page::before {
  content: '';
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0, 0, 0, 0.04) 2px,
    rgba(0, 0, 0, 0.04) 4px
  );
  pointer-events: none;
  z-index: 1;
}

.login-page::after {
  content: '';
  position: absolute;
  top: -50%;
  left: -50%;
  width: 200%;
  height: 200%;
  background: radial-gradient(
    ellipse at 50% 70%,
    rgba(232, 145, 45, 0.06) 0%,
    transparent 50%
  );
  pointer-events: none;
  animation: ambientPulse 6s ease-in-out infinite alternate;
}

@keyframes ambientPulse {
  from { opacity: 0.7; }
  to { opacity: 1; }
}

.login-container {
  width: 100%;
  max-width: 380px;
  position: relative;
  z-index: 2;
}

.login-brand {
  text-align: center;
  margin-bottom: var(--sp-3xl);
}

.login-logo {
  width: 56px;
  height: 56px;
  background: var(--accent);
  border-radius: var(--radius-xl);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-2xl);
  color: #000;
  margin-bottom: var(--sp-lg);
  box-shadow: 0 0 24px var(--accent-glow), 0 0 60px rgba(232, 145, 45, 0.12);
}

.login-title {
  font-family: var(--font-mono);
  font-size: var(--fs-2xl);
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -1px;
}

.login-subtitle {
  font-size: var(--fs-base);
  color: var(--text-secondary);
  margin-top: var(--sp-sm);
}

.login-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  padding: var(--sp-2xl);
  box-shadow: var(--phosphor-shadow);
  transition: border-color var(--transition-base);
}

.login-card:focus-within {
  border-color: rgba(232, 145, 45, 0.2);
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
}

.remember-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: var(--fs-sm);
  color: var(--text-secondary);
  cursor: pointer;
}

.checkbox-label input[type='checkbox'] {
  accent-color: var(--accent);
  width: 14px;
  height: 14px;
}

.login-btn {
  width: 100%;
  height: 42px;
  font-size: var(--fs-md);
}

.login-error {
  color: var(--danger);
  font-size: var(--fs-sm);
  text-align: center;
}

.login-footer {
  text-align: center;
  margin-top: var(--sp-2xl);
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.login-footer .version {
  font-family: var(--font-mono);
}

@media (max-width: 767px) {
  .login-container {
    max-width: 100%;
  }
  .login-card {
    padding: var(--sp-xl);
  }
}
</style>
