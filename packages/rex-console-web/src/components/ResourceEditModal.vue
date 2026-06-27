<template>
  <div v-if="visible" class="modal-overlay" @click.self="close">
    <div class="modal-content">
      <div class="modal-header">
        <span>{{ t('resource.edit') }}</span>
        <button @click="close">×</button>
      </div>

      <div class="modal-body">
        <!-- Loading -->
        <div v-if="loading" class="loading-state">
          <p>{{ t('common.loading') }}</p>
        </div>

        <template v-else>
          <div class="form-group">
            <label class="form-label">{{ t('resource.name') }}</label>
            <input v-model="form.name" class="form-input" :placeholder="t('resource.namePlaceholder')" required />
          </div>

          <!-- Redis -->
          <template v-if="form.protocol === 'redis'">
            <div class="form-row">
              <div class="form-group flex-2">
                <label class="form-label">{{ t('resource.redis.host') }}</label>
                <input v-model="redisConfig.host" class="form-input" placeholder="127.0.0.1" required />
              </div>
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.redis.port') }}</label>
                <input v-model="redisConfig.port" class="form-input" placeholder="6379" />
              </div>
            </div>
            <div class="form-group">
              <label class="form-label">{{ t('resource.redis.password') }}</label>
              <input v-model="redisConfig.password" class="form-input" type="password" :placeholder="t('resource.redis.passwordPlaceholder')" />
            </div>
            <div class="form-row">
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.redis.db') }}</label>
                <input v-model="redisConfig.db" class="form-input" type="number" min="0" max="15" placeholder="0" />
              </div>
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.redis.name') }}</label>
                <input v-model="redisConfig.name" class="form-input" :placeholder="t('resource.redis.namePlaceholder')" />
              </div>
            </div>
          </template>

          <!-- Docker -->
          <template v-else-if="form.protocol === 'docker'">
            <div class="form-group">
              <label class="form-label">{{ t('resource.docker.mode') }}</label>
              <div class="auth-toggle">
                <button type="button" class="auth-btn" :class="{ active: dockerConfig.mode === 'unix' }" @click="dockerConfig.mode = 'unix'">Unix Socket</button>
                <button type="button" class="auth-btn" :class="{ active: dockerConfig.mode === 'tcp' }" @click="dockerConfig.mode = 'tcp'">TCP</button>
              </div>
            </div>
            <template v-if="dockerConfig.mode === 'unix'">
              <div class="form-group">
                <label class="form-label">{{ t('resource.docker.socketPath') }}</label>
                <input v-model="dockerConfig.socketPath" class="form-input" placeholder="/var/run/docker.sock" />
              </div>
            </template>
            <template v-else>
              <div class="form-row">
                <div class="form-group flex-2">
                  <label class="form-label">{{ t('resource.docker.host') }}</label>
                  <input v-model="dockerConfig.host" class="form-input" placeholder="127.0.0.1" required />
                </div>
                <div class="form-group flex-1">
                  <label class="form-label">{{ t('resource.docker.port') }}</label>
                  <input v-model="dockerConfig.port" class="form-input" placeholder="2375" />
                </div>
              </div>
            </template>
            <div class="form-group">
              <label class="form-label">{{ t('resource.docker.name') }}</label>
              <input v-model="dockerConfig.name" class="form-input" :placeholder="t('resource.docker.namePlaceholder')" />
            </div>
          </template>

          <!-- SQLite -->
          <template v-else-if="form.protocol === 'sqlite'">
            <div class="form-group">
              <label class="form-label">{{ t('resource.sqlite.dbPath') }}</label>
              <input v-model="sqliteConfig.dbPath" class="form-input" placeholder="/path/to/database.db" required />
            </div>
            <div class="form-group">
              <label class="form-label">{{ t('resource.sqlite.name') }}</label>
              <input v-model="sqliteConfig.name" class="form-input" :placeholder="t('resource.sqlite.namePlaceholder')" />
            </div>
          </template>

          <!-- S3 -->
          <template v-else-if="form.protocol === 's3'">
            <div class="form-group">
              <label class="form-label">{{ t('resource.s3.endpoint') }}</label>
              <input v-model="s3Config.endpoint" class="form-input" placeholder="https://s3.amazonaws.com" required />
            </div>
            <div class="form-row">
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.s3.accessKey') }}</label>
                <input v-model="s3Config.accessKey" class="form-input" required />
              </div>
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.s3.secretKey') }}</label>
                <input v-model="s3Config.secretKey" class="form-input" type="password" required />
              </div>
            </div>
            <div class="form-row">
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.s3.region') }}</label>
                <input v-model="s3Config.region" class="form-input" placeholder="us-east-1" />
              </div>
              <div class="form-group flex-1">
                <label class="form-label">{{ t('resource.s3.bucket') }}</label>
                <input v-model="s3Config.bucket" class="form-input" />
              </div>
            </div>
            <div class="form-group">
              <label class="form-label">
                <input v-model="s3Config.forcePathStyle" type="checkbox" />
                {{ t('resource.s3.forcePathStyle') }}
              </label>
            </div>
            <div class="form-group">
              <label class="form-label">{{ t('resource.s3.name') }}</label>
              <input v-model="s3Config.name" class="form-input" :placeholder="t('resource.s3.namePlaceholder')" />
            </div>
          </template>

          <!-- SSH / SQL -->
          <template v-else>
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
                <button type="button" class="auth-btn" :class="{ active: sshConfig.auth === 'password' }" @click="sshConfig.auth = 'password'">
                  {{ t('resource.ssh.password') }}
                </button>
                <button type="button" class="auth-btn" :class="{ active: sshConfig.auth === 'key' }" @click="sshConfig.auth = 'key'">
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
          </template>
        </template>
      </div>

      <div class="modal-footer">
        <button class="btn btn-ghost" @click="close">{{ t('common.cancel') }}</button>
        <button class="btn btn-primary" :disabled="loading || saving || !canSubmit" @click="submitUpdate">
          {{ saving ? t('common.saving') : t('resource.save') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { getResource, updateResource, type Resource } from '@/api/env'
import { useSidebar } from '@/composables/useSidebar'

const props = defineProps<{
  visible: boolean
  envId: string
  resourceId: string
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const { t } = useI18n()
const { fetchEnvs } = useSidebar()

const loading = ref(false)
const saving = ref(false)

const form = reactive({
  protocol: '',
  name: '',
})

const sshConfig = reactive({
  host: '',
  port: '22',
  user: 'root',
  auth: 'password' as 'password' | 'key',
  password: '',
  keyFile: '',
})

const redisConfig = reactive({
  host: '127.0.0.1',
  port: '6379',
  password: '',
  db: '0',
  name: '',
})

const dockerConfig = reactive({
  mode: 'unix' as 'unix' | 'tcp',
  host: '127.0.0.1',
  port: '2375',
  socketPath: '/var/run/docker.sock',
  name: '',
})

const sqliteConfig = reactive({
  dbPath: '',
  name: '',
})

const s3Config = reactive({
  endpoint: '',
  accessKey: '',
  secretKey: '',
  region: '',
  bucket: '',
  forcePathStyle: true,
  name: '',
})

function parseConfigJson(configJson: string) {
  try { return JSON.parse(configJson) } catch { return {} }
}

function loadResource(cfg: Record<string, any>, protocol: string) {
  if (protocol === 'redis') {
    redisConfig.host = cfg.host || '127.0.0.1'
    redisConfig.port = String(cfg.port || 6379)
    redisConfig.password = cfg.password || ''
    redisConfig.db = String(cfg.db ?? 0)
    redisConfig.name = cfg.name || ''
  } else if (protocol === 'docker') {
    const host = cfg.host || ''
    if (host.startsWith('unix://')) {
      dockerConfig.mode = 'unix'
      dockerConfig.socketPath = host.slice(7)
    } else {
      dockerConfig.mode = 'tcp'
      const match = host.match(/^tcp:\/\/([^:]+):(\d+)$/)
      dockerConfig.host = match?.[1] || '127.0.0.1'
      dockerConfig.port = match?.[2] || '2375'
    }
    dockerConfig.name = cfg.name || ''
  } else if (protocol === 'sqlite') {
    sqliteConfig.dbPath = cfg.db_path || ''
    sqliteConfig.name = cfg.name || ''
  } else if (protocol === 's3') {
    s3Config.endpoint = cfg.endpoint || ''
    s3Config.accessKey = cfg.access_key || ''
    s3Config.secretKey = cfg.secret_key || ''
    s3Config.region = cfg.region || ''
    s3Config.bucket = cfg.bucket || ''
    s3Config.forcePathStyle = cfg.force_path_style ?? true
    s3Config.name = cfg.name || ''
  } else {
    sshConfig.host = cfg.host || ''
    sshConfig.port = String(cfg.port || 22)
    sshConfig.user = cfg.username || cfg.user || ''
    if (cfg.auth) {
      sshConfig.auth = cfg.auth.type || 'password'
      sshConfig.password = cfg.auth.password || ''
      sshConfig.keyFile = cfg.auth.private_key_path || ''
    }
  }
}

watch(() => props.visible, async (v) => {
  if (!v || !props.resourceId) return
  loading.value = true
  try {
    const resource = await getResource(props.envId, props.resourceId)
    form.protocol = resource.protocol
    form.name = resource.name
    loadResource(parseConfigJson(resource.config_json), resource.protocol)
  } catch {
    emit('update:visible', false)
  } finally {
    loading.value = false
  }
})

function buildConfigJson() {
  const port = parseInt(sshConfig.port) || (form.protocol === 'mysql' ? 3306 : form.protocol === 'postgresql' ? 5432 : 22)
  if (form.protocol === 'redis') {
    const db = Number(redisConfig.db)
    return JSON.stringify({ host: redisConfig.host, port: Number(redisConfig.port) || 6379, password: redisConfig.password || null, db: db >= 0 && db <= 15 ? db : 0, name: redisConfig.name || null })
  }
  if (form.protocol === 'docker') {
    const host = dockerConfig.mode === 'unix' ? `unix://${dockerConfig.socketPath}` : `tcp://${dockerConfig.host}:${dockerConfig.port || '2375'}`
    return JSON.stringify({ host, name: dockerConfig.name || null })
  }
  if (form.protocol === 'sqlite') return JSON.stringify({ db_path: sqliteConfig.dbPath, name: sqliteConfig.name || null })
  if (form.protocol === 's3') {
    return JSON.stringify({ endpoint: s3Config.endpoint, access_key: s3Config.accessKey, secret_key: s3Config.secretKey, region: s3Config.region || null, bucket: s3Config.bucket || null, force_path_style: s3Config.forcePathStyle, name: s3Config.name || null })
  }
  if (form.protocol === 'mysql' || form.protocol === 'postgresql') {
    return JSON.stringify({ host: sshConfig.host, port, user: sshConfig.user, password: sshConfig.password })
  }
  return JSON.stringify({
    host: sshConfig.host, port, username: sshConfig.user,
    auth: { type: sshConfig.auth, ...(sshConfig.auth === 'password' ? { password: sshConfig.password } : { private_key_path: sshConfig.keyFile }) },
  })
}

const canSubmit = computed(() => {
  if (!form.name.trim()) return false
  if (form.protocol === 'redis') return !!redisConfig.host.trim()
  if (form.protocol === 'docker') return dockerConfig.mode === 'unix' ? !!dockerConfig.socketPath.trim() : !!dockerConfig.host.trim()
  if (form.protocol === 'sqlite') return !!sqliteConfig.dbPath.trim()
  if (form.protocol === 's3') return !!s3Config.endpoint.trim() && !!s3Config.accessKey.trim() && !!s3Config.secretKey.trim()
  return !!sshConfig.host.trim() && !!sshConfig.user.trim()
})

function close() {
  emit('update:visible', false)
}

async function submitUpdate() {
  saving.value = true
  try {
    await updateResource(props.envId, props.resourceId, {
      name: form.name,
      config_json: buildConfigJson(),
    })
    fetchEnvs()
    close()
  } catch {
    // 静默处理
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--bg-panel);
  border-radius: 8px;
  width: 90%;
  max-width: 560px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.modal-header span { font-weight: 600; font-size: 16px; color: var(--text-primary); }

.modal-header button {
  background: transparent; border: none; font-size: 24px; cursor: pointer;
  color: var(--text-secondary); width: 32px; height: 32px;
  display: flex; align-items: center; justify-content: center;
}

.modal-header button:hover { color: var(--text-primary); background: var(--bg-hover); border-radius: 4px; }

.modal-body {
  flex: 1;
  overflow: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid var(--border);
}

.form-row { display: flex; gap: 12px; }
.flex-1 { flex: 1; }
.flex-2 { flex: 2; }

.auth-toggle {
  display: flex; gap: 0; border: 1px solid var(--border); border-radius: 6px; overflow: hidden;
}

.auth-btn {
  flex: 1; padding: 8px 12px; background: var(--bg-deep); border: none;
  color: var(--text-secondary); font-size: 13px; cursor: pointer; transition: all 0.15s;
}

.auth-btn.active { background: var(--accent); color: #000; font-weight: 600; }

.loading-state { text-align: center; padding: 40px; color: var(--text-muted); }

@media (max-width: 767px) {
  .modal-content { width: 95%; max-height: 85vh; }
  .form-row { flex-direction: column; }
}
</style>
