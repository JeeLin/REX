<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import Button from '@/components/ui/Button.vue'
import Input from '@/components/ui/Input.vue'
import Checkbox from '@/components/ui/Checkbox.vue'

const { t } = useI18n()
const auth = useAuthStore()
const router = useRouter()
const route = useRoute()

declare const __APP_VERSION__: string
const appVersion = __APP_VERSION__

const password = ref('')
const remember = ref(true)
const errorMsg = ref('')
const bootLines = ref<string[]>([])
const bootDone = ref(false)

const BOOT_SEQUENCE = [
  `> REX Hub v${appVersion}`,
  '> Initializing secure connection...',
  '> TLS 1.3 handshake OK',
  '> Loading design system...',
  '> Ready.',
]

onMounted(async () => {
  for (const line of BOOT_SEQUENCE) {
    await new Promise(r => setTimeout(r, 200))
    bootLines.value.push(line)
  }
  await new Promise(r => setTimeout(r, 300))
  bootDone.value = true
})

async function handleLogin() {
  if (!password.value) return
  errorMsg.value = ''
  try {
    await auth.login(password.value, remember.value)
    const redirect = (route.query.redirect as string) || '/workspace'
    // 防止开放重定向：只允许相对路径
    const safeRedirect = redirect.startsWith('/') && !redirect.startsWith('//') ? redirect : '/workspace'
    router.push(safeRedirect)
  } catch (e: unknown) {
    errorMsg.value = e instanceof Error ? e.message : t('login.loginFailed')
  }
}
</script>

<template>
  <div class="login">
    <!-- CRT 效果层 -->
    <div class="crt-scanlines" />
    <div class="crt-flicker" />
    <div class="bg-grid" />
    <div class="bg-glow" />

    <!-- 主内容区 -->
    <div class="login-wrapper" :class="{ 'login-wrapper--visible': bootDone }">
      <!-- 终端启动序列 -->
      <div v-if="!bootDone" class="boot-terminal mono">
        <div v-for="(line, i) in bootLines" :key="i" class="boot-line">{{ line }}</div>
        <span class="boot-cursor">█</span>
      </div>

      <!-- 登录卡片 -->
      <Transition name="card-reveal">
        <div v-if="bootDone" class="login-card">
          <!-- 品牌区 -->
          <div class="brand">
            <div class="brand-icon">
              <span class="brand-bracket">[</span>
              <span class="brand-name mono">REX</span>
              <span class="brand-bracket">]</span>
            </div>
            <div class="brand-tagline">{{ t('login.subtitle') }}</div>
          </div>

          <!-- 表单区 -->
          <form class="login-form" @submit.prevent="handleLogin">
            <div class="field">
              <label class="field-label mono">PASSWORD</label>
              <Input
                v-model="password"
                type="password"
                size="lg"
                placeholder="••••••••"
                autocomplete="current-password"
                autofocus
              >
                <template #prefix>
                  <span class="field-prompt mono">$</span>
                </template>
              </Input>
            </div>

            <div class="login-options">
              <Checkbox v-model="remember" :label="t('login.rememberMe')" />
            </div>

            <Transition name="error-slide">
              <div v-if="errorMsg" class="error-box">
                <span class="error-icon">✗</span>
                <span>{{ errorMsg }}</span>
              </div>
            </Transition>

            <Button
              variant="primary"
              size="lg"
              :disabled="auth.loading || !password"
              class="login-btn"
            >
              <span v-if="auth.loading" class="btn-spinner" />
              {{ auth.loading ? t('login.signingIn') : t('login.signIn') }}
            </Button>
          </form>

          <!-- 底部状态栏 -->
          <div class="status-bar mono">
            <span class="status-item">
              <span class="status-dot status-dot--online" />
              SECURE
            </span>
            <span class="status-sep">|</span>
            <span class="status-item">v{{ appVersion }}</span>
            <span class="status-sep">|</span>
            <span class="status-item">self-hosted</span>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
/* ========== 基础布局 ========== */
.login {
  height: 100%;
  display: grid;
  place-items: center;
  background: var(--bg-deep);
  position: relative;
  overflow: hidden;
}

/* ========== CRT 效果 ========== */
.crt-scanlines {
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0, 0, 0, 0.15) 2px,
    rgba(0, 0, 0, 0.15) 4px
  );
  pointer-events: none;
  z-index: 10;
}
.crt-flicker {
  position: absolute;
  inset: 0;
  background: radial-gradient(
    ellipse at center,
    transparent 60%,
    rgba(0, 0, 0, 0.4) 100%
  );
  pointer-events: none;
  z-index: 11;
}

/* ========== 背景装饰 ========== */
.bg-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(232, 145, 45, 0.04) 1px, transparent 1px),
    linear-gradient(90deg, rgba(232, 145, 45, 0.04) 1px, transparent 1px);
  background-size: 48px 48px;
  pointer-events: none;
  z-index: 0;
}
.bg-glow {
  position: absolute;
  width: 500px;
  height: 500px;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: radial-gradient(circle, rgba(232, 145, 45, 0.06) 0%, transparent 70%);
  pointer-events: none;
  z-index: 0;
  animation: glow-pulse 4s ease-in-out infinite;
}

/* ========== 启动终端 ========== */
.boot-terminal {
  position: absolute;
  top: 20%;
  left: 50%;
  transform: translateX(-50%);
  width: 400px;
  padding: var(--space-6);
  background: rgba(13, 17, 23, 0.9);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  z-index: 5;
}
.boot-line {
  font-size: var(--text-sm);
  color: var(--success);
  line-height: 1.8;
  opacity: 0;
  animation: boot-fade-in 0.3s ease forwards;
}
.boot-cursor {
  color: var(--accent);
  animation: blink 1s step-end infinite;
  font-size: var(--text-sm);
}

/* ========== 主内容区 ========== */
.login-wrapper {
  position: relative;
  z-index: 5;
  opacity: 0;
  transition: opacity 0.5s ease;
}
.login-wrapper--visible {
  opacity: 1;
}

/* ========== 登录卡片 ========== */
.login-card {
  width: 400px;
  padding: var(--space-8) var(--space-8) var(--space-6);
  background: rgba(28, 33, 40, 0.8);
  backdrop-filter: blur(16px);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow:
    var(--shadow-lg),
    0 0 60px rgba(232, 145, 45, 0.04),
    inset 0 1px 0 rgba(255, 255, 255, 0.03);
}

/* ========== 品牌区 ========== */
.brand {
  text-align: center;
  margin-bottom: var(--space-6);
}
.brand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2px;
  margin-bottom: var(--space-2);
}
.brand-bracket {
  font-size: 36px;
  font-weight: 300;
  color: var(--text-muted);
  font-family: var(--font-mono);
}
.brand-name {
  font-size: 36px;
  font-weight: 700;
  color: var(--accent);
  letter-spacing: 4px;
  text-shadow: 0 0 20px rgba(232, 145, 45, 0.3);
}
.brand-tagline {
  font-size: var(--text-sm);
  color: var(--text-muted);
  letter-spacing: 1px;
}

/* ========== 表单 ========== */
.login-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.field-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
  letter-spacing: 1.5px;
}
.field-prompt {
  color: var(--accent);
  font-weight: 600;
}
.login-options {
  display: flex;
  align-items: center;
}
.login-btn {
  width: 100%;
  margin-top: var(--space-1);
}
.btn-spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin-right: var(--space-2);
  vertical-align: middle;
}

/* ========== 错误提示 ========== */
.error-box {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--danger-soft);
  border: 1px solid rgba(248, 81, 73, 0.3);
  border-radius: var(--radius);
  color: var(--danger);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
}
.error-icon {
  font-size: var(--text-xs);
  flex-shrink: 0;
}

/* ========== 底部状态栏 ========== */
.status-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  margin-top: var(--space-6);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-subtle);
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.status-item {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
}
.status-dot--online {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}
.status-sep {
  opacity: 0.3;
}

/* ========== 动画 ========== */
@keyframes spin {
  to { transform: rotate(360deg); }
}
@keyframes blink {
  50% { opacity: 0; }
}
@keyframes boot-fade-in {
  from { opacity: 0; transform: translateY(2px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes glow-pulse {
  0%, 100% { opacity: 0.6; }
  50% { opacity: 1; }
}

/* ========== 过渡 ========== */
.card-reveal-enter-active {
  transition: opacity 0.6s ease, transform 0.6s ease;
}
.card-reveal-enter-from {
  opacity: 0;
  transform: translateY(12px) scale(0.98);
}
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
