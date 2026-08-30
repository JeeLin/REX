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
      <!-- 品牌标识 -->
      <div class="setup-brand">
        <span class="glyph">R</span>
        <span class="brand-name">RE<b>X</b> Hub</span>
      </div>

      <h1>{{ t('setup.subtitle') }}</h1>
      <p class="setup-sub">{{ t('setup.hint') }}</p>

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

        <Transition name="error-slide">
          <div v-if="errorMsg" class="error-msg">{{ errorMsg }}</div>
        </Transition>

        <Button
          variant="primary"
          size="lg"
          :disabled="auth.loading || !password || !confirmPassword"
          class="setup-btn"
        >
          {{ auth.loading ? t('setup.setting') : t('setup.setPassword') }}
        </Button>
      </form>

      <div class="setup-foot">
        REX Hub · self-hosted
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ========== 整体布局 ========== */
.setup {
  min-height: 100%;
  display: grid;
  place-items: center;
  padding: 40px;
  background:
    radial-gradient(120% 120% at 50% 0%, rgba(232, 145, 45, 0.10), transparent 55%),
    var(--bg-page);
}

/* ========== 设置卡片 ========== */
.setup-card {
  width: 100%;
  max-width: 380px;
  padding: var(--space-8);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
}

/* ========== 品牌标识 ========== */
.setup-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: var(--space-6);
}

.setup-brand .glyph {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  background: linear-gradient(140deg, var(--accent), var(--brand-deep));
  color: var(--on-brand);
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 16px;
  box-shadow: 0 0 0 1px rgba(232, 145, 45, 0.4), 0 4px 14px rgba(232, 145, 45, 0.25);
}

.brand-name {
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 22px;
  letter-spacing: .02em;
  color: var(--text-primary);
}
.brand-name b {
  color: var(--accent);
}

/* ========== 标题与描述 ========== */
.setup-card h1 {
  font-size: 20px;
  margin: 0 0 6px;
  letter-spacing: -.02em;
  color: var(--text-primary);
}

.setup-sub {
  color: var(--text-muted);
  font-size: 13.5px;
  margin: 0 0 24px;
  line-height: 1.5;
}

/* ========== 表单 ========== */
.setup-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 500;
}

.field-input {
  width: 100%;
  height: 42px;
  padding: 0 13px;
  border-radius: var(--radius);
  border: 1px solid var(--border-strong);
  background: var(--bg-page);
  color: var(--text-primary);
  font: inherit;
  font-size: var(--text-md);
  transition: border-color var(--transition), box-shadow var(--transition);
}
.field-input::placeholder {
  color: var(--text-muted);
  opacity: 0.6;
}
.field-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

/* ========== 错误提示 ========== */
.error-msg {
  color: var(--danger);
  font-size: var(--text-sm);
  text-align: center;
  min-height: 16px;
  font-family: var(--font-mono);
}

/* ========== 设置按钮 ========== */
.setup-btn {
  width: 100%;
  margin-top: var(--space-2);
}

/* ========== 底部 ========== */
.setup-foot {
  text-align: center;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11.5px;
  margin-top: 22px;
  opacity: 0.5;
}

/* ========== 过渡 ========== */
.error-slide-enter-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.error-slide-leave-active {
  transition: opacity 0.15s ease;
}
.error-slide-enter-from {
  opacity: 0;
  transform: translateY(-4px);
}
.error-slide-leave-to {
  opacity: 0;
}
</style>
