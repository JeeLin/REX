<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { type TestConnectionResult } from '@/api/resources'
import Button from '@/components/ui/Button.vue'
import Modal from '@/components/ui/Modal.vue'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from './protocols'

const { t } = useI18n()
const props = defineProps<{
  visible: boolean
  environmentId: string
}>()

const emit = defineEmits<{
  close: []
  created: []
}>()

const store = useEnvironmentsStore()

// 从环境继承连接方式
const environment = computed(() => store.environments.find(e => e.id === props.environmentId))
const connectionMode = computed(() => environment.value?.connection_mode || 'direct')

const loading = ref(false)
const error = ref('')

// Step 1: Protocol
const protocols = [
  { id: 'ssh', descKey: 'wizard.sshDesc' },
  { id: 'sftp', descKey: 'wizard.sftpDesc' },
  { id: 'mysql', descKey: 'wizard.mysqlDesc' },
  { id: 'postgresql', descKey: 'wizard.postgresqlDesc' },
  { id: 'redis', descKey: 'wizard.redisDesc' },
  { id: 'sqlite', descKey: 'wizard.sqliteDesc' },
  { id: 's3', descKey: 'wizard.s3Desc' },
]
const selectedProtocol = ref('')

// Step 2: Basic info
const resName = ref('')
const resColor = ref('')

// Step 3: Connection details
const host = ref('')
const port = ref<number | null>(null)
const username = ref('')
const password = ref('')
const privateKey = ref('')
const databaseName = ref('')
const filePath = ref('')
const s3Endpoint = ref('')
const s3AccessKey = ref('')
const s3SecretKey = ref('')
const s3Bucket = ref('')
const s3Region = ref('')
const redisDb = ref(0)

// Test connection
const testResult = ref<TestConnectionResult | null>(null)
const testLoading = ref(false)

const defaultPorts: Record<string, number> = {
  ssh: 22, sftp: 22, mysql: 3306, postgresql: 5432, redis: 6379,
}

function selectProtocol(id: string) {
  selectedProtocol.value = id
  port.value = defaultPorts[id] ?? null
  // Auto-fill name
  if (!resName.value) {
    resName.value = id === 's3' ? 'S3 / MinIO' : id.charAt(0).toUpperCase() + id.slice(1)
  }
}

async function testConnection() {
  testLoading.value = true
  testResult.value = null
  try {
    testResult.value = await store.testConnection({
      protocol: selectedProtocol.value,
      host: selectedProtocol.value === 'sqlite' ? filePath.value : host.value,
      port: port.value,
      username: username.value,
      config_json: JSON.stringify(buildConfig()),
    })
  } catch (e: unknown) {
    testResult.value = { ok: false, error: e instanceof Error ? e.message : String(e) }
  } finally {
    testLoading.value = false
  }
}

function buildConfig(): Record<string, unknown> {
  const cfg: Record<string, unknown> = {}
  if (['ssh', 'sftp'].includes(selectedProtocol.value)) {
    if (password.value) cfg.password = password.value
    if (privateKey.value) cfg.private_key = privateKey.value
  } else if (['mysql', 'postgresql'].includes(selectedProtocol.value)) {
    if (password.value) cfg.password = password.value
    if (databaseName.value) cfg.database_name = databaseName.value
  } else if (selectedProtocol.value === 'redis') {
    if (password.value) cfg.password = password.value
    cfg.db = redisDb.value
  } else if (selectedProtocol.value === 'sqlite') {
    cfg.file_path = filePath.value
  } else if (selectedProtocol.value === 's3') {
    cfg.endpoint = s3Endpoint.value
    cfg.access_key = s3AccessKey.value
    cfg.secret_key = s3SecretKey.value
    cfg.bucket = s3Bucket.value
    cfg.region = s3Region.value || 'us-east-1'
  }
  return cfg
}

async function submit() {
  loading.value = true
  error.value = ''
  try {
    await store.createResource(props.environmentId, {
      name: resName.value.trim(),
      protocol: selectedProtocol.value,
      host: selectedProtocol.value === 'sqlite' ? filePath.value : (selectedProtocol.value === 's3' ? s3Endpoint.value : host.value),
      port: port.value,
      username: username.value || undefined,
      config_json: JSON.stringify(buildConfig()),
      color: resColor.value || undefined,
    })
    emit('created')
    reset()
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

function reset() {
  selectedProtocol.value = ''
  resName.value = ''
  resColor.value = ''
  host.value = ''
  port.value = null
  username.value = ''
  password.value = ''
  privateKey.value = ''
  databaseName.value = ''
  filePath.value = ''
  s3Endpoint.value = ''
  s3AccessKey.value = ''
  s3SecretKey.value = ''
  s3Bucket.value = ''
  s3Region.value = ''
  redisDb.value = 0
  testResult.value = null
  error.value = ''
}

function handleClose() {
  reset()
  emit('close')
}

const colorOptions = [
  '', '#3FB950', '#58A6FF', '#8B5CF6', '#F85149', '#D29922', '#E8912D', '#F0883E', '#8B949E',
]
</script>

<template>
  <Modal :model-value="visible" @update:model-value="handleClose">
    <template #title>{{ t('wizard.createResource') }}</template>

    <div class="wizard-single-page">
      <!-- Protocol Selection -->
      <div class="form-label">
        <span>{{ t('wizard.protocol') }}</span>
        <div class="protocol-grid">
          <button
            v-for="p in protocols"
            :key="p.id"
            class="protocol-card"
            :class="{ selected: selectedProtocol === p.id }"
            @click="selectProtocol(p.id)"
          >
            <span class="protocol-icon" :style="{ color: PROTOCOL_COLORS[p.id] }">{{ PROTOCOL_ICONS[p.id] }}</span>
            <span class="protocol-name">{{ p.id === 's3' ? 'S3 / MinIO' : p.id.charAt(0).toUpperCase() + p.id.slice(1) }}</span>
          </button>
        </div>
      </div>

      <!-- Basic Info (shown after protocol selected) -->
      <template v-if="selectedProtocol">
        <label class="form-label">
          <span>{{ t('common.name') }}</span>
          <input v-model="resName" type="text" class="form-input" placeholder="e.g. Web Server" autofocus />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.color') }}</span>
          <div class="color-picker">
            <button
              v-for="c in colorOptions"
              :key="c"
              class="color-dot"
              :class="{ selected: resColor === c }"
              :style="{ background: c || 'var(--border)' }"
              @click="resColor = c"
            />
          </div>
        </label>

        <!-- Connection Details -->
        <!-- SSH / SFTP -->
        <template v-if="['ssh', 'sftp'].includes(selectedProtocol)">
          <label class="form-label">
            <span>{{ t('wizard.host') }}</span>
            <input v-model="host" type="text" class="form-input" placeholder="e.g. 192.168.1.100" />
          </label>
        <label class="form-label">
          <span>{{ t('wizard.port') }}</span>
          <input v-model.number="port" type="number" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.username') }}</span>
          <input v-model="username" type="text" class="form-input" placeholder="e.g. root" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.password') }}</span>
          <input v-model="password" type="password" class="form-input" placeholder="(optional if using key)" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.privateKey') }}</span>
          <textarea v-model="privateKey" class="form-input form-textarea" placeholder="(optional) Paste private key" rows="3"></textarea>
        </label>
      </template>

      <!-- MySQL / PostgreSQL -->
      <template v-if="['mysql', 'postgresql'].includes(selectedProtocol)">
        <label class="form-label">
          <span>{{ t('wizard.host') }}</span>
          <input v-model="host" type="text" class="form-input" placeholder="e.g. 10.0.0.5" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.port') }}</span>
          <input v-model.number="port" type="number" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.username') }}</span>
          <input v-model="username" type="text" class="form-input" placeholder="e.g. root" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.password') }}</span>
          <input v-model="password" type="password" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.database') }}</span>
          <input v-model="databaseName" type="text" class="form-input" placeholder="(optional)" />
        </label>
      </template>

      <!-- Redis -->
      <template v-if="selectedProtocol === 'redis'">
        <label class="form-label">
          <span>{{ t('wizard.host') }}</span>
          <input v-model="host" type="text" class="form-input" placeholder="e.g. 127.0.0.1" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.port') }}</span>
          <input v-model.number="port" type="number" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.password') }}</span>
          <input v-model="password" type="password" class="form-input" placeholder="(optional)" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.redisDb') }}</span>
          <input v-model.number="redisDb" type="number" class="form-input" min="0" max="15" />
        </label>
      </template>

      <!-- SQLite -->
      <template v-if="selectedProtocol === 'sqlite'">
        <label class="form-label">
          <span>{{ t('wizard.filePath') }}</span>
          <input v-model="filePath" type="text" class="form-input" placeholder="/path/to/database.sqlite" />
        </label>
      </template>

      <!-- S3 -->
      <template v-if="selectedProtocol === 's3'">
        <label class="form-label">
          <span>{{ t('wizard.s3Endpoint') }}</span>
          <input v-model="s3Endpoint" type="text" class="form-input" placeholder="https://s3.amazonaws.com" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.s3AccessKey') }}</span>
          <input v-model="s3AccessKey" type="text" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.s3SecretKey') }}</span>
          <input v-model="s3SecretKey" type="password" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.s3Bucket') }}</span>
          <input v-model="s3Bucket" type="text" class="form-input" />
        </label>
        <label class="form-label">
          <span>{{ t('wizard.s3Region') }}</span>
          <input v-model="s3Region" type="text" class="form-input" placeholder="us-east-1" />
        </label>
      </template>

      <!-- Test connection -->
      <div class="test-section">
        <Button variant="secondary" size="sm" :loading="testLoading" @click="testConnection">
          {{ t('wizard.testConnection') }}
        </Button>
        <span v-if="testResult?.ok" class="test-ok">✓ {{ t('wizard.testSuccess') }} ({{ testResult.latency_ms }}ms)</span>
        <span v-else-if="testResult && !testResult.ok" class="test-fail">✕ {{ testResult.error }}</span>
      </div>
    </template>
    </div>

    <div v-if="error" class="form-error" style="margin-bottom: var(--space-3)">{{ error }}</div>

    <!-- Actions -->
    <div class="form-actions">
      <Button variant="secondary" @click="handleClose">{{ t('common.cancel') }}</Button>
      <div style="flex:1"></div>
      <Button variant="primary" :loading="loading" :disabled="!selectedProtocol || !resName.trim()" @click="submit">
        {{ t('wizard.create') }}
      </Button>
    </div>
  </Modal>
</template>

<style scoped>
.step-indicator {
  font-size: var(--text-xs);
  color: var(--text-muted);
  font-weight: 400;
  margin-left: var(--space-2);
}
.protocol-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}
.protocol-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-4) var(--space-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-deep);
  cursor: pointer;
  transition: all var(--transition);
}
.protocol-card:hover {
  border-color: var(--text-muted);
}
.protocol-card.selected {
  border-color: var(--accent);
  background: rgba(232, 145, 45, 0.05);
}
.protocol-icon {
  font-size: 24px;
  font-family: var(--font-mono);
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-hover);
}
.protocol-name {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}
.protocol-desc {
  font-size: var(--text-xs);
}
.step-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}
.form-label {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.form-input {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
.form-input:focus {
  border-color: var(--accent);
}
.form-textarea {
  font-family: var(--font-mono);
  resize: vertical;
}
.color-picker {
  display: flex;
  gap: var(--space-2);
}
.color-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color var(--transition);
}
.color-dot:hover {
  border-color: var(--text-muted);
}
.color-dot.selected {
  border-color: var(--text-primary);
}
.test-section {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-top: var(--space-2);
}
.test-ok {
  color: var(--success);
  font-size: var(--text-sm);
}
.test-fail {
  color: var(--danger);
  font-size: var(--text-sm);
}
.step-confirm {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}
.confirm-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.confirm-label {
  color: var(--text-muted);
  font-weight: 500;
}
.mono {
  font-family: var(--font-mono);
}
.muted {
  color: var(--text-muted);
}
.form-error {
  color: var(--danger);
  font-size: var(--text-sm);
  margin-top: var(--space-2);
}
.form-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-4);
}
</style>
