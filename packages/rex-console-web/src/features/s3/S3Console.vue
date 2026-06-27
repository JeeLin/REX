<template>
  <div class="s3-console">
    <!-- 顶部状态栏 -->
    <div class="s3-topbar">
      <span class="s3-status-dot" :class="{ connected: session.connected.value }" />
      <span class="s3-topbar-label">S3</span>
      <span class="s3-topbar-name">{{ resourceName }}</span>
      <span v-if="endpoint" class="s3-topbar-state">{{ endpoint }}</span>
      <div class="s3-topbar-spacer" />
      <button
        v-if="!session.connected.value"
        class="s3-btn s3-btn-connect"
        :disabled="connecting"
        @click="handleConnect"
      >
        {{ connecting ? t('s3.connecting') : t('s3.connect') }}
      </button>
      <button
        v-else
        class="s3-btn s3-btn-disconnect"
        @click="handleDisconnect"
      >
        {{ t('s3.disconnect') }}
      </button>
    </div>

    <!-- 错误提示 -->
    <div v-if="session.error.value" class="s3-error-banner">
      {{ session.error.value }}
    </div>

    <!-- 未连接欢迎页 -->
    <div v-if="!session.connected.value && !session.error.value" class="s3-welcome">
      <p>{{ t('s3.welcome') }}</p>
    </div>

    <!-- 已连接：Bucket 列表 + Object 浏览 -->
    <template v-else>
      <!-- Bucket 列表 -->
      <BucketList
        v-if="!currentBucket"
        :buckets="buckets"
        :loading="bucketsLoading"
        :selected="null"
        @select="selectBucket"
        @refresh="refreshBuckets"
      />

      <!-- Object 浏览 -->
      <ObjectBrowser
        v-else
        :bucket="currentBucket"
        :prefix="currentPrefix"
        :items="objects"
        :loading="objectsLoading"
        @navigate="navigateTo"
        @upload="triggerUpload"
        @refresh="refreshObjects"
        @download="downloadFile"
        @delete="deleteFile"
      />
    </template>

    <!-- 隐藏的文件输入 -->
    <input
      ref="fileInput"
      type="file"
      style="display: none"
      @change="handleFileUpload"
    />
    <ConfirmDialog
      :visible="showDeleteConfirm"
      :title="t('confirm.deleteTitle')"
      :message="deleteConfirmMsg"
      :confirm-label="t('common.delete')"
      :cancel-label="t('common.cancel')"
      danger
      @confirm="doDeleteFile"
      @cancel="showDeleteConfirm = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useS3Session, type S3BucketInfo, type S3ObjectInfo } from './useS3Session'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import BucketList from './BucketList.vue'
import ObjectBrowser from './ObjectBrowser.vue'

const props = defineProps<{
  resourceId: string
  resourceName: string
}>()

const { t } = useI18n()

const session = useS3Session(() => props.resourceId)

const connecting = ref(false)
const bucketsLoading = ref(false)
const objectsLoading = ref(false)
const buckets = ref<S3BucketInfo[]>([])
const currentBucket = ref('')
const currentPrefix = ref('')
const objects = ref<S3ObjectInfo[]>([])
const fileInput = ref<HTMLInputElement | null>(null)
const showDeleteConfirm = ref(false)
const deleteConfirmMsg = ref('')
let pendingDeleteKey = ''

const endpoint = ref('')

// ── 连接/断开 ────────────────────────────────────────────
async function handleConnect() {
  connecting.value = true
  try {
    await session.connect()
    const info = session.serverInfo.value
    if (info) {
      endpoint.value = info['endpoint'] || ''
    }
    await refreshBuckets()
  } catch {
    // error is set in session.error
  } finally {
    connecting.value = false
  }
}

function handleDisconnect() {
  session.disconnect()
  currentBucket.value = ''
  currentPrefix.value = ''
  buckets.value = []
  objects.value = []
  endpoint.value = ''
}

// ── Bucket 操作 ─────────────────────────────────────────
async function refreshBuckets() {
  if (!session.connected.value) return
  bucketsLoading.value = true
  try {
    buckets.value = await session.listBuckets()
  } catch {
    // error surfaced via session.error
  } finally {
    bucketsLoading.value = false
  }
}

function selectBucket(name: string) {
  currentBucket.value = name
  currentPrefix.value = ''
  refreshObjects()
}

// ── Object 操作 ─────────────────────────────────────────
async function refreshObjects() {
  if (!session.connected.value || !currentBucket.value) return
  objectsLoading.value = true
  try {
    objects.value = await session.listObjects(currentBucket.value, currentPrefix.value)
  } catch {
    // error surfaced via session.error
  } finally {
    objectsLoading.value = false
  }
}

function navigateTo(prefix: string) {
  currentPrefix.value = prefix
  refreshObjects()
}

// ── 上传 ────────────────────────────────────────────────
function triggerUpload() {
  fileInput.value?.click()
}

async function handleFileUpload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file || !currentBucket.value) return

  const key = currentPrefix.value + file.name
  try {
    const buffer = await file.arrayBuffer()
    await session.uploadObject(currentBucket.value, key, buffer)
    await refreshObjects()
  } catch (err) {
    session.error.value = err instanceof Error ? err.message : String(err)
  }

  input.value = ''
}

// ── 下载 ────────────────────────────────────────────────
async function downloadFile(item: S3ObjectInfo) {
  try {
    const buffer = await session.downloadObject(currentBucket.value, item.key)
    const blob = new Blob([buffer])
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = item.key.split('/').pop() || item.key
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  } catch (err) {
    session.error.value = err instanceof Error ? err.message : String(err)
  }
}

// ── 删除 ────────────────────────────────────────────────
function deleteFile(item: S3ObjectInfo) {
  const name = item.key.split('/').filter(Boolean).pop()
  pendingDeleteKey = item.key
  deleteConfirmMsg.value = t('s3.confirmDelete', { name })
  showDeleteConfirm.value = true
}

async function doDeleteFile() {
  showDeleteConfirm.value = false
  try {
    await session.deleteObject(currentBucket.value, pendingDeleteKey)
    await refreshObjects()
  } catch (err) {
    session.error.value = err instanceof Error ? err.message : String(err)
  }
}

// ── 自动连接 ─────────────────────────────────────────────
onMounted(() => {
  if (!session.connected.value) {
    session.connect().then(() => {
      const info = session.serverInfo.value
      if (info) {
        endpoint.value = info['endpoint'] || ''
      }
      refreshBuckets()
    }).catch(() => {})
  }
})

onUnmounted(() => {
  session.disconnect()
})
</script>

<style scoped>
.s3-console {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
}

/* 顶部状态栏 */
.s3-topbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  font-size: 13px;
  flex-shrink: 0;
}

.s3-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f85149;
}

.s3-status-dot.connected { background: #3fb950; }

.s3-topbar-label { font-weight: 600; color: #e8912d; }
.s3-topbar-name { color: var(--text-secondary); }
.s3-topbar-state { color: var(--text-secondary); font-size: 12px; }
.s3-topbar-spacer { flex: 1; }

/* 按钮 */
.s3-btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-primary);
  color: var(--text-primary);
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
}

.s3-btn:hover:not(:disabled) { background: var(--bg-hover); }
.s3-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.s3-btn-connect { border-color: #3fb950; color: #3fb950; }
.s3-btn-disconnect { border-color: #f85149; color: #f85149; }

/* 错误 */
.s3-error-banner {
  color: #f85149;
  padding: 8px 12px;
  border-bottom: 1px solid #f8514933;
  background: #f8514911;
  font-size: 12px;
  flex-shrink: 0;
}

/* 欢迎 */
.s3-welcome {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: 14px;
  font-family: inherit;
}
</style>
