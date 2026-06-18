<template>
  <div class="resource-wizard">
    <div class="wizard-header">
      <router-link :to="`/environments/${envId}`" class="btn btn-ghost btn-sm">
        ← {{ t('common.back') }}
      </router-link>
      <h2 class="page-title">{{ t('resource.wizard') }}</h2>
    </div>

    <!-- Step indicator -->
    <div class="steps">
      <div class="step" :class="{ active: step === 1, done: step > 1 }">
        <span class="step-num">{{ step > 1 ? '✓' : '1' }}</span>
        <span class="step-label">{{ t('resource.step1') }}</span>
      </div>
      <div class="step-line" :class="{ active: step > 1 }"></div>
      <div class="step" :class="{ active: step === 2, done: step > 2 }">
        <span class="step-num">{{ step > 2 ? '✓' : '2' }}</span>
        <span class="step-label">{{ t('resource.step2') }}</span>
      </div>
      <div class="step-line" :class="{ active: step > 2 }"></div>
      <div class="step" :class="{ active: step === 3, done: step > 3 }">
        <span class="step-num">{{ step > 3 ? '✓' : '3' }}</span>
        <span class="step-label">{{ t('resource.step3') }}</span>
      </div>
      <div class="step-line" :class="{ active: step > 3 }"></div>
      <div class="step" :class="{ active: step === 4 }">
        <span class="step-num">4</span>
        <span class="step-label">{{ t('resource.step4') }}</span>
      </div>
    </div>

    <!-- Step 1: Protocol -->
    <div v-if="step === 1" class="step-content">
      <p class="step-desc">{{ t('resource.selectProtocol') }}</p>
      <div class="protocol-grid">
        <div
          v-for="proto in protocols"
          :key="proto.id"
          class="protocol-card"
          :class="{ selected: form.protocol === proto.id }"
          @click="selectProtocol(proto.id)"
        >
          <div class="proto-icon" :style="{ color: proto.color }">{{ proto.icon }}</div>
          <div class="proto-name">{{ proto.name }}</div>
        </div>
      </div>
    </div>

    <!-- Step 2: Basic Info -->
    <div v-if="step === 2" class="step-content">
      <form class="wizard-form card" @submit.prevent="nextStep">
        <div class="form-group">
          <label class="form-label">{{ t('resource.name') }}</label>
          <input v-model="form.name" class="form-input" :placeholder="t('resource.namePlaceholder')" required />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('resource.connectionMode') }}</label>
          <div class="mode-cards">
            <div
              class="mode-card"
              :class="{ active: form.connection_mode === 'agent_proxy' }"
              @click="form.connection_mode = 'agent_proxy'"
            >
              <div class="mode-title">⬡ {{ t('env.agentProxy') }}</div>
            </div>
            <div
              class="mode-card"
              :class="{ active: form.connection_mode === 'direct' }"
              @click="form.connection_mode = 'direct'"
            >
              <div class="mode-title">→ {{ t('env.direct') }}</div>
            </div>
          </div>
        </div>
      </form>
    </div>

    <!-- Step 3: Connection Details (SSH example) -->
    <div v-if="step === 3" class="step-content">
      <form class="wizard-form card" @submit.prevent="nextStep">
        <div class="form-row">
          <div class="form-group flex-2">
            <label class="form-label">{{ t('resource.ssh.host') }}</label>
            <input v-model="sshConfig.host" class="form-input" :placeholder="t('resource.ssh.hostPlaceholder')" required />
          </div>
          <div class="form-group flex-1">
            <label class="form-label">{{ t('resource.ssh.port') }}</label>
            <input v-model="sshConfig.port" class="form-input" :placeholder="t('resource.ssh.portPlaceholder')" />
          </div>
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('resource.ssh.user') }}</label>
          <input v-model="sshConfig.user" class="form-input" :placeholder="t('resource.ssh.userPlaceholder')" required />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t('resource.ssh.auth') }}</label>
          <div class="auth-toggle">
            <button
              type="button"
              class="auth-btn"
              :class="{ active: sshConfig.auth === 'password' }"
              @click="sshConfig.auth = 'password'"
            >
              {{ t('resource.ssh.password') }}
            </button>
            <button
              type="button"
              class="auth-btn"
              :class="{ active: sshConfig.auth === 'key' }"
              @click="sshConfig.auth = 'key'"
            >
              {{ t('resource.ssh.keyFile') }}
            </button>
          </div>
          <input
            v-if="sshConfig.auth === 'password'"
            v-model="sshConfig.password"
            class="form-input"
            type="password"
            placeholder="••••••••"
          />
          <input
            v-else
            v-model="sshConfig.keyFile"
            class="form-input"
            placeholder="~/.ssh/id_rsa"
          />
        </div>
      </form>
    </div>

    <!-- Step 4: Complete -->
    <div v-if="step === 4" class="step-content">
      <div class="complete-card card">
        <div class="complete-icon">✓</div>
        <h3 class="complete-title">{{ t('resource.created') }}</h3>
        <p class="complete-desc">{{ form.name }} · {{ form.protocol.toUpperCase() }}</p>
        <div class="complete-actions">
          <router-link :to="`/environments/${envId}`" class="btn btn-primary">
            {{ t('common.back') }}
          </router-link>
        </div>
      </div>
    </div>

    <!-- Navigation -->
    <div v-if="step < 4" class="wizard-nav">
      <button v-if="step > 1" class="btn btn-ghost" @click="prevStep">
        {{ t('common.prev') }}
      </button>
      <div v-else></div>
      <button
        v-if="step < 4"
        class="btn btn-primary"
        :disabled="!canNext"
        @click="step === 3 ? submitResource() : nextStep()"
      >
        {{ step === 3 ? t('resource.createBtn') : t('common.next') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import client from '@/api/client'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const envId = route.params.envId as string
const step = ref(1)

const protocols = [
  { id: 'ssh', name: 'SSH', icon: '$', color: 'var(--success)' },
  { id: 'sftp', name: 'SFTP', icon: '📁', color: 'var(--accent-purple)' },
  { id: 'mysql', name: 'MySQL', icon: 'dB', color: 'var(--info)' },
  { id: 'postgresql', name: 'PostgreSQL', icon: 'pg', color: 'var(--accent-purple)' },
  { id: 'redis', name: 'Redis', icon: 'R', color: 'var(--danger)' },
  { id: 'docker', name: 'Docker', icon: '🐳', color: 'var(--info)' },
  { id: 'sqlite', name: 'SQLite', icon: 'S', color: 'var(--warning)' },
  { id: 's3', name: 'S3', icon: '☁', color: 'var(--accent)' },
]

const form = reactive({
  protocol: '',
  name: '',
  connection_mode: 'direct',
  config_json: '',
})

const sshConfig = reactive({
  host: '',
  port: '22',
  user: 'root',
  auth: 'password' as 'password' | 'key',
  password: '',
  keyFile: '',
})

const canNext = computed(() => {
  if (step.value === 1) return !!form.protocol
  if (step.value === 2) return !!form.name.trim()
  if (step.value === 3) return !!sshConfig.host.trim() && !!sshConfig.user.trim()
  return false
})

function selectProtocol(id: string) {
  form.protocol = id
  form.name = protocols.find(p => p.id === id)?.name || ''
  step.value = 2
}

function nextStep() {
  if (step.value < 4) step.value++
}

function prevStep() {
  if (step.value > 1) step.value--
}

async function submitResource() {
  form.config_json = JSON.stringify(sshConfig)
  try {
    await client.post(`/environments/${envId}/resources`, form)
    step.value = 4
  } catch {
    // 静默处理
  }
}
</script>

<style scoped>
.wizard-header {
  display: flex;
  align-items: center;
  gap: var(--sp-lg);
  margin-bottom: var(--sp-xl);
}

.steps {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0;
  margin-bottom: var(--sp-2xl);
  padding: 0 var(--sp-xl);
}

.step {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  color: var(--text-muted);
}

.step.active {
  color: var(--accent);
}

.step.done {
  color: var(--success);
}

.step-num {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid currentColor;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: var(--fs-xs);
}

.step.active .step-num {
  background: var(--accent);
  border-color: var(--accent);
  color: #000;
}

.step.done .step-num {
  background: var(--success);
  border-color: var(--success);
  color: #000;
}

.step-label {
  font-size: var(--fs-sm);
  font-weight: 500;
}

.step-line {
  width: 40px;
  height: 2px;
  background: var(--border);
  margin: 0 var(--sp-sm);
  transition: background var(--transition-base);
}

.step-line.active {
  background: var(--accent);
}

.step-content {
  max-width: 640px;
  margin: 0 auto;
}

.step-desc {
  text-align: center;
  color: var(--text-secondary);
  margin-bottom: var(--sp-xl);
}

.protocol-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--sp-md);
}

.protocol-card {
  background: var(--bg-surface);
  border: 2px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-xl);
  cursor: pointer;
  transition: all var(--transition-base);
  text-align: center;
}

.protocol-card:hover {
  border-color: var(--accent);
}

.protocol-card.selected {
  border-color: var(--accent);
  background: rgba(232, 145, 45, 0.05);
  box-shadow: 0 0 20px rgba(232, 145, 45, 0.08);
}

.proto-icon {
  font-size: 24px;
  font-family: var(--font-mono);
  font-weight: 700;
  margin-bottom: var(--sp-sm);
}

.proto-name {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-sm);
}

.wizard-form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xl);
}

.form-row {
  display: flex;
  gap: var(--sp-md);
}

.flex-1 { flex: 1; }
.flex-2 { flex: 2; }

.mode-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--sp-md);
}

.mode-card {
  background: var(--bg-deep);
  border: 2px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--sp-lg);
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
}

.mode-title {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-sm);
}

.auth-toggle {
  display: flex;
  gap: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.auth-btn {
  flex: 1;
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-deep);
  border: none;
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.auth-btn.active {
  background: var(--accent);
  color: #000;
  font-weight: 600;
}

.complete-card {
  text-align: center;
  padding: var(--sp-3xl);
}

.complete-icon {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: var(--success);
  color: #000;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  font-weight: 700;
  margin-bottom: var(--sp-lg);
}

.complete-title {
  font-family: var(--font-mono);
  font-size: var(--fs-lg);
  font-weight: 600;
  margin-bottom: var(--sp-sm);
}

.complete-desc {
  color: var(--text-secondary);
  margin-bottom: var(--sp-xl);
}

.complete-actions {
  display: flex;
  justify-content: center;
}

.wizard-nav {
  display: flex;
  justify-content: space-between;
  margin-top: var(--sp-xl);
  padding: 0 var(--sp-xl);
}

@media (max-width: 767px) {
  .protocol-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  .form-row {
    flex-direction: column;
  }
}
</style>
