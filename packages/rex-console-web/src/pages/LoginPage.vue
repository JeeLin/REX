<script setup lang="ts">
import { ref } from 'vue'
import Button from '@/components/ui/Button.vue'

const username = ref('')
const password = ref('')
const loading = ref(false)

function handleLogin() {
  if (!username.value || !password.value) return
  loading.value = true
  setTimeout(() => {
    loading.value = false
    window.location.href = '/workspace'
  }, 800)
}
</script>

<template>
  <div class="login">
    <div class="login-card">
      <div class="login-brand mono">
        REX<span class="accent">Hub</span>
      </div>
      <div class="login-subtitle">Remote Resource Management</div>

      <form class="login-form" @submit.prevent="handleLogin">
        <div class="field">
          <label class="field-label mono">Username</label>
          <input
            v-model="username"
            type="text"
            class="field-input"
            placeholder="admin"
            autocomplete="username"
          />
        </div>
        <div class="field">
          <label class="field-label mono">Password</label>
          <input
            v-model="password"
            type="password"
            class="field-input"
            placeholder="••••••••"
            autocomplete="current-password"
          />
        </div>
        <Button
          variant="primary"
          size="lg"
          :disabled="loading || !username || !password"
          style="width: 100%; margin-top: var(--space-4)"
        >
          {{ loading ? 'Signing in...' : 'Sign in' }}
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
.login-footer {
  text-align: center;
  margin-top: var(--space-6);
  font-size: var(--text-xs);
}
</style>
