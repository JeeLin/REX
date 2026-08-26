<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useEnvironmentsStore } from '@/stores/environments'
import { type TestConnectionResult, type NewResource } from '@/api/resources'
import Button from '@/components/ui/Button.vue'
import Modal from '@/components/ui/Modal.vue'
import { PROTOCOL_ICONS, PROTOCOL_COLORS } from './protocols'
import { type SipAccountForm, type SipAccountView, resolveActiveAccount } from '@/features/sip/types'

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

const loading = ref(false)
const error = ref('')

// Step 1: Protocol
// v0.70.7：mysql/postgresql/sqlite 合并为单一「SQL」资源；创建时可选子类（dialect），
// 留空则在连接时由后端自动识别（端口预判 → 双线缆握手回退 → SELECT VERSION() 确认）。
const protocols = [
  { id: 'ssh', descKey: 'wizard.sshDesc' },
  { id: 'sftp', descKey: 'wizard.sftpDesc' },
  { id: 'sql', descKey: 'wizard.sqlDesc' },
  { id: 'redis', descKey: 'wizard.redisDesc' },
  { id: 's3', descKey: 'wizard.s3Desc' },
  { id: 'sip', descKey: 'wizard.sipDesc' },
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
const initScript = ref('')
const databaseName = ref('')
const filePath = ref('')
const s3Endpoint = ref('')
const s3AccessKey = ref('')
const s3SecretKey = ref('')
const s3Bucket = ref('')
const s3Region = ref('')
const redisDb = ref(0)
// SIP：名称（= 资源名，仅展示分组）+ 多账户，每个账户自带 server/port/transport 与凭据。
const sipAccounts = ref<SipAccountForm[]>([
  { id: 'a1', server: '', port: null, transport: 'udp', username: '', password: '', displayName: '' },
])
const sipActiveAccount = ref('a1')
// 单调递增计数器，避免删除账户后再新增时 id 重复（长度派生 id 会碰撞）。
const sipAccountSeq = ref(1)

function addSipAccount() {
  sipAccountSeq.value += 1
  const id = `a${sipAccountSeq.value}`
  sipAccounts.value.push({
    id,
    server: '',
    port: null,
    transport: 'udp',
    username: '',
    password: '',
    displayName: '',
  })
  sipActiveAccount.value = id
}

function removeSipAccount(id: string) {
  if (sipAccounts.value.length <= 1) return
  sipAccounts.value = sipAccounts.value.filter((a) => a.id !== id)
  if (sipActiveAccount.value === id) {
    sipActiveAccount.value = sipAccounts.value[0]?.id ?? 'a1'
  }
}

function setActiveSipAccount(id: string) {
  sipActiveAccount.value = id
}

// Test connection
const testResult = ref<TestConnectionResult | null>(null)
const testLoading = ref(false)

const defaultPorts: Record<string, number> = {
  ssh: 22, sftp: 22, mysql: 3306, postgresql: 5432, redis: 6379, sip: 5060,
}

// v0.70.7：SQL 资源可选子类（dialect）。默认空 = 连接时自动识别；选 mysql/postgresql 时预填端口。
const selectedSubtype = ref('')

// SQL 子类 → 默认端口（仅在未显式选端口时使用）。
const subtypeDefaultPorts: Record<string, number> = { mysql: 3306, postgresql: 5432 }

function selectProtocol(id: string) {
  selectedProtocol.value = id
  selectedSubtype.value = ''
  port.value = defaultPorts[id] ?? null
  // Auto-fill name
  if (!resName.value) {
    resName.value = id === 's3' ? 'S3 / MinIO' : id.charAt(0).toUpperCase() + id.slice(1)
  }
}

// SQL 子类切换时预填默认端口；用户已手动改过非默认端口则不覆盖。
function onSqlSubtypeChange() {
  const dflt = subtypeDefaultPorts[selectedSubtype.value] ?? 3306
  if (!port.value || port.value === 3306 || port.value === 5432) {
    port.value = dflt
  }
}

async function testConnection() {
  testLoading.value = true
  testResult.value = null
  try {
    testResult.value = await store.testConnection({
      protocol: selectedProtocol.value,
      host: resourceHost(),
      port: port.value,
      username: username.value,
      config_json: JSON.stringify(buildConfig()),
      environment_id: props.environmentId,
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
    if (initScript.value.trim()) cfg.initScript = initScript.value
  } else if (selectedProtocol.value === 'sql') {
    if (selectedSubtype.value === 'sqlite') {
      cfg.file_path = filePath.value
    } else {
      // 显式选 mysql/postgresql 时，凭据随子类保存；自动识别（空子类）也带凭据以加速探测。
      if (password.value) cfg.password = password.value
      if (databaseName.value) cfg.database_name = databaseName.value
    }
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
  } else if (selectedProtocol.value === 'sip') {
    // SipProfile 形状：名称（仅展示）+ 多账户，每个账户自带 server/port/transport 与凭据。
    const accounts = toSipAccounts()
    // 生效账户用共享解析规则（active 或 first），与后端 load_sip_conn 语义一致，
    // 确保 activeAccount 指向被保留的账户（过滤掉空账户后可能失效）。
    const active = resolveActiveAccount(accounts, sipActiveAccount.value)
    cfg.accounts = accounts
    if (active) cfg.activeAccount = active.id
  }
  return cfg
}

// 由账户表单派生 SipAccountView[]：过滤空 username 账户并归一化字段。
// buildConfig 与 resourceHost 共用，避免两处重复变换产生漂移。
function toSipAccounts(): SipAccountView[] {
  return sipAccounts.value
    .filter((a) => a.username.trim())
    .map((a) => {
      const acc: SipAccountView = {
        id: a.id,
        server: a.server.trim(),
        port: a.port ?? 5060,
        transport: a.transport,
        username: a.username.trim(),
      }
      if (a.password) acc.password = a.password
      if (a.displayName.trim()) acc.displayName = a.displayName.trim()
      return acc
    })
}

// 资源顶层 host：sqlite（含 SQL 子类）用 file_path；s3 用 endpoint；sip 取生效账户 server。
// 直接由账户列表派生，避免维护易过期的 sipHost ref。
function resourceHost(): string {
  if (selectedProtocol.value === 'sqlite') return filePath.value
  if (selectedProtocol.value === 'sql' && selectedSubtype.value === 'sqlite') return filePath.value
  if (selectedProtocol.value === 's3') return s3Endpoint.value
  if (selectedProtocol.value === 'sip') {
    const active = resolveActiveAccount(toSipAccounts(), sipActiveAccount.value)
    return active?.server ?? ''
  }
  return host.value
}

async function submit() {
  loading.value = true
  error.value = ''
  if (
    selectedProtocol.value === 'sip' &&
    !sipAccounts.value.some((a) => a.username.trim())
  ) {
    error.value = 'sip 资源至少需要一个有效账户（填写用户名）'
    loading.value = false
    return
  }
  try {
    const cfg = buildConfig()
    const payload: NewResource = {
      name: resName.value.trim(),
      protocol: selectedProtocol.value,
      host: resourceHost(),
      port: port.value,
      username: username.value || undefined,
      config_json: JSON.stringify(cfg),
      color: resColor.value || undefined,
    }
    // v0.70.7：SQL 资源显式选子类（dialect）时随请求发送，跳过后端自动识别。
    if (selectedProtocol.value === 'sql' && selectedSubtype.value) {
      payload.subtype = selectedSubtype.value
    }
    await store.createResource(props.environmentId, payload)
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
  initScript.value = ''
  databaseName.value = ''
  filePath.value = ''
  s3Endpoint.value = ''
  s3AccessKey.value = ''
  s3SecretKey.value = ''
  s3Bucket.value = ''
  s3Region.value = ''
  redisDb.value = 0
  selectedSubtype.value = ''
  sipAccounts.value = [
    { id: 'a1', server: '', port: null, transport: 'udp', username: '', password: '', displayName: '' },
  ]
  sipActiveAccount.value = 'a1'
  testResult.value = null
  error.value = ''
}

function handleClose() {
  reset()
  emit('close')
}

const colorOptions = [
  '', 'var(--proto-ssh)', 'var(--proto-mysql)', 'var(--proto-postgresql)', 'var(--proto-redis)', 'var(--proto-sqlite)', 'var(--proto-s3)', 'var(--accent-hover)', 'var(--text-muted)',
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
          <label class="form-label">
            <span>{{ t('wizard.initScript') }}</span>
            <textarea v-model="initScript" class="form-input form-textarea" placeholder="(optional) cd /data/logs&#10;echo ready" rows="3"></textarea>
          </label>
        </template>

        <!-- SQL (unified MySQL / PostgreSQL / SQLite) -->
        <template v-if="selectedProtocol === 'sql'">
          <!-- 子类（dialect）选择：留空 = 连接时自动识别 -->
          <label class="form-label">
            <span>{{ t('wizard.sqlSubtype') }}</span>
            <select v-model="selectedSubtype" class="form-input" @change="onSqlSubtypeChange">
              <option value="">{{ t('wizard.autoDetect') }}</option>
              <option value="mysql">MySQL</option>
              <option value="postgresql">PostgreSQL</option>
              <option value="sqlite">SQLite</option>
            </select>
          </label>
          <!-- SQLite 子类：本地文件路径 -->
          <template v-if="selectedSubtype === 'sqlite'">
            <label class="form-label">
              <span>{{ t('wizard.filePath') }}</span>
              <input v-model="filePath" type="text" class="form-input" placeholder="/path/to/database.sqlite" />
            </label>
          </template>
          <!-- 服务端 dialects（MySQL / PostgreSQL / 自动识别）：host/port/username/password/database -->
          <template v-else>
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

        <!-- SIP：名称（资源名）+ 多账户，每账户自带 server/port/transport -->
        <template v-if="selectedProtocol === 'sip'">
          <!-- 多账户 -->
          <div class="sip-accounts">
            <div class="sip-accounts-head">
              <span>{{ t('wizard.sipAccounts') }}</span>
              <Button variant="secondary" size="sm" @click="addSipAccount">{{ t('wizard.addAccount') }}</Button>
            </div>
            <div
              v-for="acc in sipAccounts"
              :key="acc.id"
              class="sip-account-card"
              :class="{ active: sipActiveAccount === acc.id }"
            >
              <div class="sip-account-head">
                <label class="sip-account-radio">
                  <input
                    type="radio"
                    :name="'sip-active-account'"
                    :checked="sipActiveAccount === acc.id"
                    @change="setActiveSipAccount(acc.id)"
                  />
                  <span>{{ t('wizard.activeAccount') }}</span>
                </label>
                <button
                  v-if="sipAccounts.length > 1"
                  class="sip-account-remove"
                  @click="removeSipAccount(acc.id)"
                >✕</button>
              </div>
              <label class="form-label">
                <span>{{ t('wizard.sipServer') }}</span>
                <input v-model="acc.server" type="text" class="form-input" placeholder="e.g. sip.example.com" />
              </label>
              <div class="sip-account-row">
                <label class="form-label">
                  <span>{{ t('wizard.port') }}</span>
                  <input v-model.number="acc.port" type="number" class="form-input" />
                </label>
                <label class="form-label">
                  <span>{{ t('wizard.sipTransport') }}</span>
                  <select v-model="acc.transport" class="form-input">
                    <option value="udp">UDP</option>
                    <option value="tcp">TCP</option>
                    <option value="tls">TLS</option>
                  </select>
                </label>
              </div>
              <label class="form-label">
                <span>{{ t('wizard.username') }}</span>
                <input v-model="acc.username" type="text" class="form-input" placeholder="e.g. 1000" />
              </label>
              <label class="form-label">
                <span>{{ t('wizard.password') }}</span>
                <input v-model="acc.password" type="password" class="form-input" placeholder="(optional for anonymous)" />
              </label>
              <label class="form-label">
                <span>{{ t('wizard.sipDisplayName') }}</span>
                <input v-model="acc.displayName" type="text" class="form-input" placeholder="(optional) e.g. Alice" />
              </label>
            </div>
          </div>
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

.sip-accounts {
  border-top: 1px solid var(--border);
  margin-top: var(--space-3);
  padding-top: var(--space-3);
}

.sip-accounts-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-2);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.sip-account-card {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-3);
  margin-bottom: var(--space-2);
  background: var(--bg-surface);
}

.sip-account-card.active {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.sip-account-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-2);
}

.sip-account-radio {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  cursor: pointer;
}

.sip-account-remove {
  background: transparent;
  border: none;
  color: var(--danger);
  cursor: pointer;
  font-size: var(--font-size-md);
  padding: 0 var(--space-1);
}
.sip-account-row {
  display: flex;
  gap: var(--space-2);
}
.sip-account-row .form-label {
  flex: 1;
}
</style>
