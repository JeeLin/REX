<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import Tabs from '@/components/ui/Tabs.vue'
import Input from '@/components/ui/Input.vue'
import Select from '@/components/ui/Select.vue'
import Button from '@/components/ui/Button.vue'

interface ResourceProps {
  name: string
  protocol: string
  host?: string
  port?: string
  user?: string
  password?: string
  privateKey?: string
  passphrase?: string
  region?: string
  encoding: string
  scrollback: number
  cursorStyle: string
  cursorBlink: boolean
  theme: string
  fontSize: number
  opacity: number
  backgroundImage: string
  keepalive: boolean
  keepaliveInterval: number
  color: string
  notes: string
}

const props = defineProps<{
  show: boolean
  resource?: Partial<ResourceProps>
}>()

const emit = defineEmits<{
  'update:show': [value: boolean]
  save: [data: ResourceProps]
}>()

const form = ref<ResourceProps>({
  name: '',
  protocol: 'ssh',
  host: '',
  port: '',
  user: '',
  password: '',
  privateKey: '',
  passphrase: '',
  region: '',
  encoding: 'UTF-8',
  scrollback: 10000,
  cursorStyle: 'block',
  cursorBlink: true,
  theme: 'default',
  fontSize: 14,
  opacity: 100,
  backgroundImage: 'none',
  keepalive: true,
  keepaliveInterval: 60,
  color: '',
  notes: '',
})

watch(() => props.resource, (r) => {
  if (r) form.value = { ...form.value, ...r }
}, { immediate: true })

const activeTab = ref('connection')

// Terminal/Appearance/Keepalive only for SSH/SFTP
const isTerminalProtocol = computed(() => ['ssh', 'sftp'].includes(form.value.protocol))

const propTabs = computed(() => {
  const base = ['connection', 'auth']
  if (isTerminalProtocol.value) {
    base.push('terminal', 'appearance', 'keepalive')
  }
  return base
})

// Reset active tab when protocol changes
watch(() => form.value.protocol, () => {
  if (!propTabs.value.includes(activeTab.value)) {
    activeTab.value = 'connection'
  }
})

const encodings = [
  { label: 'UTF-8', value: 'UTF-8' },
  { label: 'GBK', value: 'GBK' },
  { label: 'ISO-8859-1', value: 'ISO-8859-1' },
]

const cursorStyles = [
  { label: 'Block', value: 'block' },
  { label: 'Underline', value: 'underline' },
  { label: 'Bar', value: 'bar' },
]

const themes = [
  { label: 'Default', value: 'default' },
  { label: 'Ubuntu', value: 'ubuntu' },
  { label: 'Solarized Dark', value: 'solarized-dark' },
]

const bgImageOptions = [
  { label: 'None', value: 'none' },
  { label: 'Grid', value: 'grid' },
  { label: 'Dots', value: 'dots' },
  { label: 'Gradient', value: 'gradient' },
]

const authMethods = [
  { label: 'Password', value: 'password' },
  { label: 'Key File', value: 'keyfile' },
]
const authMethod = ref('password')

function getDefaultPort(): string {
  const ports: Record<string, string> = {
    ssh: '22', sql: '3306', mysql: '3306', redis: '6379', postgresql: '5432', sftp: '22', sqlite: '', s3: '443',
  }
  return ports[form.value.protocol] || '22'
}

function onSave() {
  emit('save', { ...form.value })
  emit('update:show', false)
}
</script>

<template>
  <Modal :model-value="show" :title="`Properties — ${form.name}`" width="560px" @update:model-value="emit('update:show', $event)">
    <Tabs v-model="activeTab" :tabs="propTabs">
      <template #item="{ tab }">
        <span class="mono" style="text-transform: capitalize">{{ tab }}</span>
      </template>

      <div class="props-tab-content">
        <!-- Connection -->
        <template v-if="activeTab === 'connection'">
          <div class="props-field">
            <label class="props-label">Name</label>
            <Input v-model="form.name" size="sm" />
          </div>
          <div class="props-field">
            <label class="props-label">Protocol</label>
            <Select
              v-model="form.protocol" :options="[
                { label: 'SSH', value: 'ssh' },
                { label: 'SQL', value: 'sql' },
                { label: 'MySQL', value: 'mysql' },
                { label: 'PostgreSQL', value: 'postgresql' },
                { label: 'Redis', value: 'redis' },
                { label: 'SFTP', value: 'sftp' },
                { label: 'SQLite', value: 'sqlite' },
                { label: 'S3', value: 's3' },
              ]" size="sm"
            />
          </div>
          <!-- SSH/SFTP/SQL: Host + Port -->
          <template v-if="['ssh', 'sftp', 'mysql', 'postgresql', 'redis', 'sql'].includes(form.protocol)">
            <div class="props-row">
              <div class="props-field props-field--grow">
                <label class="props-label">Host</label>
                <Input v-model="form.host" size="sm" placeholder="10.0.1.5" />
              </div>
              <div class="props-field" style="width: 100px">
                <label class="props-label">Port</label>
                <Input :model-value="form.port || getDefaultPort()" size="sm" placeholder="22" @update:model-value="form.port = $event" />
              </div>
            </div>
          </template>

          <!-- SQLite: File path -->
          <template v-if="form.protocol === 'sqlite'">
            <div class="props-field">
              <label class="props-label">File Path</label>
              <Input v-model="form.host" size="sm" placeholder="/path/to/database.db" />
            </div>
          </template>

          <!-- S3: Endpoint + Bucket + Region -->
          <template v-if="form.protocol === 's3'">
            <div class="props-field">
              <label class="props-label">Endpoint</label>
              <Input v-model="form.host" size="sm" placeholder="s3.amazonaws.com" />
            </div>
            <div class="props-field">
              <label class="props-label">Bucket</label>
              <Input v-model="form.port" size="sm" placeholder="my-bucket" />
            </div>
            <div class="props-field">
              <label class="props-label">Region</label>
              <Input v-model="form.region" size="sm" placeholder="us-east-1" />
            </div>
          </template>

          <!-- MySQL/PostgreSQL/SQL: Database -->
          <template v-if="['mysql', 'postgresql', 'sql'].includes(form.protocol)">
            <div class="props-field">
              <label class="props-label">Database</label>
              <Input v-model="form.passphrase" size="sm" placeholder="default" />
            </div>
          </template>

          <!-- Redis: Database number -->
          <template v-if="form.protocol === 'redis'">
            <div class="props-field">
              <label class="props-label">Database</label>
              <Input :model-value="String(form.scrollback)" size="sm" placeholder="0" @update:model-value="form.scrollback = Number($event)" />
            </div>
          </template>
          <div class="props-field">
            <label class="props-label">Color Tag</label>
            <div class="props-color-row">
              <span class="props-color-none" :class="{ 'props-color--active': !form.color }" @click="form.color = ''">None</span>
              <button v-for="c in ['var(--proto-redis)','var(--proto-ssh)','var(--proto-mysql)','var(--proto-sqlite)','var(--proto-postgresql)','var(--proto-s3)']" :key="c" class="props-color-dot" :class="{ 'props-color--active': form.color === c }" :style="{ background: c }" @click="form.color = c" />
            </div>
          </div>
        </template>

        <!-- Auth -->
        <template v-if="activeTab === 'auth'">
          <!-- SSH: Full auth options (password + key file) -->
          <template v-if="form.protocol === 'ssh'">
            <div class="props-field">
              <label class="props-label">Auth Method</label>
              <Select v-model="authMethod" :options="authMethods" size="sm" />
            </div>
            <div class="props-field">
              <label class="props-label">Username</label>
              <Input v-model="form.user" size="sm" placeholder="root" />
            </div>
            <template v-if="authMethod === 'password'">
              <div class="props-field">
                <label class="props-label">Password</label>
                <Input v-model="form.password" size="sm" placeholder="••••••" />
              </div>
            </template>
            <template v-else>
              <div class="props-field">
                <label class="props-label">Private Key Path</label>
                <Input v-model="form.privateKey" size="sm" placeholder="/home/user/.ssh/id_rsa" />
              </div>
              <div class="props-field">
                <label class="props-label">Passphrase</label>
                <Input v-model="form.passphrase" size="sm" placeholder="••••••" />
              </div>
            </template>
          </template>

          <!-- MySQL/PostgreSQL/SQL/Redis/SFTP: Username + Password -->
          <template v-if="['mysql', 'postgresql', 'redis', 'sftp', 'sql'].includes(form.protocol)">
            <div class="props-field">
              <label class="props-label">Username</label>
              <Input v-model="form.user" size="sm" placeholder="root" />
            </div>
            <div class="props-field">
              <label class="props-label">Password</label>
              <Input v-model="form.password" size="sm" placeholder="••••••" />
            </div>
          </template>

          <!-- S3/SQLite: No auth needed -->
          <template v-if="['s3', 'sqlite'].includes(form.protocol)">
            <div class="props-field">
              <p class="props-hint">{{ form.protocol === 's3' ? 'S3 authentication is configured via access keys on the server.' : 'SQLite databases are file-based and do not require authentication.' }}</p>
            </div>
          </template>
        </template>

        <!-- Terminal -->
        <template v-if="activeTab === 'terminal'">
          <div class="props-field">
            <label class="props-label">Encoding</label>
            <Select v-model="form.encoding" :options="encodings" size="sm" />
          </div>
          <div class="props-field">
            <label class="props-label">Scrollback Lines</label>
            <Input :model-value="String(form.scrollback)" size="sm" placeholder="10000" @update:model-value="form.scrollback = Number($event)" />
          </div>
          <div class="props-row">
            <div class="props-field props-field--grow">
              <label class="props-label">Cursor Style</label>
              <Select v-model="form.cursorStyle" :options="cursorStyles" size="sm" />
            </div>
            <div class="props-field" style="width: 120px">
              <label class="props-label">Blink</label>
              <label class="props-toggle">
                <input v-model="form.cursorBlink" type="checkbox" />
                <span class="props-toggle-slider" />
              </label>
            </div>
          </div>
        </template>

        <!-- Appearance -->
        <template v-if="activeTab === 'appearance'">
          <div class="props-field">
            <label class="props-label">Theme</label>
            <Select v-model="form.theme" :options="themes" size="sm" />
          </div>
          <div class="props-field">
            <label class="props-label">Font Size (px)</label>
            <Input :model-value="String(form.fontSize)" size="sm" placeholder="14" @update:model-value="form.fontSize = Number($event)" />
          </div>
          <div class="props-field">
            <label class="props-label">Background Opacity (%)</label>
            <Input :model-value="String(form.opacity)" size="sm" placeholder="100" @update:model-value="form.opacity = Number($event)" />
          </div>
          <div class="props-field">
            <label class="props-label">Background Image</label>
            <Select v-model="form.backgroundImage" :options="bgImageOptions" size="sm" />
          </div>
        </template>

        <!-- Keepalive -->
        <template v-if="activeTab === 'keepalive'">
          <div class="props-field">
            <label class="props-label">Send Keepalive</label>
            <label class="props-toggle">
              <input v-model="form.keepalive" type="checkbox" />
              <span class="props-toggle-slider" />
            </label>
          </div>
          <div v-if="form.keepalive" class="props-field">
            <label class="props-label">Interval (seconds)</label>
            <Input :model-value="String(form.keepaliveInterval)" size="sm" placeholder="60" @update:model-value="form.keepaliveInterval = Number($event)" />
          </div>
        </template>

        <!-- Notes (always available via color field in Connection) -->
        <div v-if="activeTab === 'connection'" class="props-field">
          <label class="props-label">Notes</label>
          <textarea v-model="form.notes" class="props-textarea" rows="3" placeholder="Add notes about this resource..." />
        </div>
      </div>
    </Tabs>

    <template #footer>
      <Button variant="ghost" @click="emit('update:show', false)">Cancel</Button>
      <Button variant="primary" @click="onSave">Save</Button>
    </template>
  </Modal>
</template>

<style scoped>
.props-tab-content {
  padding: var(--space-4) 0;
}
.props-field {
  margin-bottom: var(--space-3);
}
.props-field--grow {
  flex: 1;
}
.props-row {
  display: flex;
  gap: var(--space-3);
}
.props-label {
  display: block;
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin-bottom: var(--space-1);
  font-weight: 500;
}
.props-textarea {
  width: 100%;
  padding: var(--space-2);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-family: var(--font-sans);
  font-size: var(--text-base);
  resize: vertical;
  outline: none;
}
.props-textarea:focus {
  border-color: var(--accent);
}
.props-color-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.props-color-none {
  font-size: var(--text-xs);
  color: var(--text-muted);
  cursor: pointer;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}
.props-color--active {
  outline: 2px solid var(--text-primary);
  outline-offset: 1px;
}
.props-color-dot {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
}
.props-toggle {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
}
.props-toggle input {
  display: none;
}
.props-toggle-slider {
  width: 36px;
  height: 20px;
  background: var(--border);
  border-radius: 10px;
  position: relative;
  transition: background var(--transition);
}
.props-toggle-slider::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  background: var(--text-primary);
  border-radius: 50%;
  transition: transform var(--transition);
}
.props-toggle input:checked + .props-toggle-slider {
  background: var(--accent);
}
.props-toggle input:checked + .props-toggle-slider::after {
  transform: translateX(16px);
}
.props-hint {
  color: var(--text-secondary);
  font-size: var(--text-xs);
  margin: 0;
}
</style>
