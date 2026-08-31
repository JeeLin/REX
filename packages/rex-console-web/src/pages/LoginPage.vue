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
    <!-- 终端启动序列覆盖层 -->
    <Transition name="boot-fade">
      <div v-if="!bootDone" class="boot-overlay">
        <div class="boot-terminal mono">
          <div v-for="(line, i) in bootLines" :key="i" class="boot-line">{{ line }}</div>
          <span class="boot-cursor">█</span>
        </div>
      </div>
    </Transition>

    <!-- LEFT: 品牌面板 -->
    <aside class="login-aside">
      <div class="scan" />
      <div class="brand">
        <span class="glyph">R</span>
        <span class="name">RE<b>X</b> Hub</span>
      </div>

      <div class="aside-points">
        <div class="aside-point">
          <div class="ico">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 7h16M4 12h10M4 17h13" /></svg>
          </div>
          <div>
            <h4>{{ t('login.pitch1Title') }}</h4>
            <p>{{ t('login.pitch1Desc') }}</p>
          </div>
        </div>
        <div class="aside-point">
          <div class="ico">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2 4 6v6c0 5 3.5 8 8 10 4.5-2 8-5 8-10V6z" /></svg>
          </div>
          <div>
            <h4>{{ t('login.pitch2Title') }}</h4>
            <p>{{ t('login.pitch2Desc') }}</p>
          </div>
        </div>
        <div class="aside-point">
          <div class="ico">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="10" rx="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></svg>
          </div>
          <div>
            <h4>{{ t('login.pitch3Title') }}</h4>
            <p>{{ t('login.pitch3Desc') }}</p>
          </div>
        </div>
      </div>

      <div class="aside-foot">// remote exchange · v{{ appVersion }} · self-hosted</div>
    </aside>

    <!-- RIGHT: 登录表单 -->
    <main class="login-main">
      <div class="login-card">
        <div class="top">
          <div class="brand brand--sm">
            <span class="glyph">R</span>
            <span class="name">RE<b>X</b></span>
          </div>
          <button class="lang-btn mono">{{ t('login.langSwitch') }}</button>
        </div>

        <h1>{{ t('login.title') }}</h1>
        <p class="sub">{{ t('login.subtitle') }}</p>

        <form class="login-form" @submit.prevent="handleLogin">
          <div class="field">
            <label class="field-label mono">{{ t('login.password') }}</label>
            <Input
              v-model="password"
              type="password"
              size="lg"
              placeholder="••••••••"
              autocomplete="current-password"
              class="login-input"
              autofocus
            >
              <template #prefix>
                <svg class="field-lock-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="3" y="11" width="18" height="10" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                </svg>
              </template>
            </Input>
          </div>

          <div class="login-options">
            <Checkbox v-model="remember" :label="t('login.rememberMe')" />
          </div>

          <p v-if="errorMsg" class="error-box">
            <span class="error-icon">✗</span>
            <span>{{ errorMsg }}</span>
          </p>
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

        <p class="login-hint">
          {{ t('login.lockedOutHint') }}
        </p>

        <div class="login-foot">
          REX Hub · build {{ appVersion }} · © 2026
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
/* ========== 基础布局（左右分栏） ========== */
.login {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 1.05fr .95fr;
}

/* ========== 左侧品牌面板 ========== */
.login-aside {
  position: relative;
  overflow: hidden;
  padding: var(--space-8);
  background:
    radial-gradient(120% 120% at 0% 0%, rgba(232, 145, 45, 0.12), transparent 55%),
    radial-gradient(120% 120% at 100% 100%, rgba(88, 166, 255, 0.08), transparent 50%),
    var(--bg-sidebar);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.scan {
  position: absolute;
  inset: 0;
  pointer-events: none;
  opacity: .35;
  background: repeating-linear-gradient(to bottom, rgba(255, 255, 255, .025) 0 1px, transparent 1px 3px);
  mix-blend-mode: overlay;
}

/* ========== 品牌标识 ========== */
.brand {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.brand .glyph {
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
.brand .name {
  font-family: var(--font-mono);
  font-weight: 700;
  letter-spacing: .02em;
  font-size: 17px;
  color: var(--text-primary);
}
.brand .name b {
  color: var(--accent);
}

/* 小号品牌（右侧卡片内） */
.brand--sm .glyph {
  width: 30px;
  height: 30px;
  font-size: 14px;
}
.brand--sm .name {
  font-size: 15px;
}

/* ========== 左侧卖点 ========== */
.aside-points {
  display: grid;
  gap: var(--space-5);
  max-width: 380px;
  position: relative;
  z-index: 1;
}
.aside-point {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}
.aside-point .ico {
  flex: none;
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  color: var(--accent);
}
.aside-point h4 {
  margin: 2px 0 3px;
  font-size: 14.5px;
  color: var(--text-primary);
}
.aside-point p {
  margin: 0;
  color: var(--text-muted);
  font-size: 13px;
}

.aside-foot {
  position: relative;
  z-index: 1;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  opacity: 0.6;
}

/* ========== 右侧登录面板 ========== */
.login-main {
  display: grid;
  place-items: center;
  padding: 40px;
  background: var(--bg-app, var(--bg-page));
}

.login-card {
  width: 100%;
  max-width: 380px;
}

.login-card .top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 30px;
}

.login-card h1 {
  font-size: 23px;
  margin: 0 0 6px;
  letter-spacing: -.02em;
  color: var(--text-primary);
}

.login-card .sub {
  color: var(--text-muted);
  font-size: 13.5px;
  margin: 0 0 26px;
}

/* ========== 语言切换按钮 ========== */
.lang-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 32px;
  padding: 0 12px;
  border-radius: var(--radius);
  border: 1px solid var(--border-strong);
  background: transparent;
  color: var(--text-primary);
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}
.lang-btn:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
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
  gap: 7px;
}

.field-label {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 500;
}

.field-lock-icon {
  color: var(--text-muted);
  flex-shrink: 0;
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

/* ========== 底部提示 ========== */
.login-hint {
  font-size: 12px;
  margin: 20px 0 0;
  text-align: center;
  line-height: 1.5;
  color: var(--text-muted);
}

.login-foot {
  text-align: center;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11.5px;
  margin-top: 26px;
  opacity: 0.5;
}

/* ========== 启动终端覆盖层 ========== */
.boot-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  background: var(--bg-deep);
}

.boot-terminal {
  width: 400px;
  padding: var(--space-6);
  background: rgba(13, 17, 23, 0.9);
  border: 1px solid var(--border);
  border-radius: var(--radius);
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

/* ========== 过渡 ========== */
.boot-fade-leave-active {
  transition: opacity 0.4s ease;
}
.boot-fade-leave-to {
  opacity: 0;
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

/* ========== 响应式 ========== */
@media (max-width: 880px) {
  .login {
    grid-template-columns: 1fr;
  }
  .login-aside {
    display: none;
  }
}
</style>
