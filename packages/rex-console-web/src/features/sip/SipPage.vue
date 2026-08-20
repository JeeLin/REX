<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Dialpad from './Dialpad.vue'
import CallState from './CallState.vue'
import { SipClient, type SipCallState, type SipServerEvent, sipCaptureApi, recordingApi } from '@/api/sip'
import {
  SipAudio,
  SipVideo,
  encodePcmFrame,
  decodeMediaFrame,
  encodeVideoFrame,
  decodeVideoFrame,
} from '@/api/sipMedia'
import { resourcesApi, type Resource } from '@/api/resources'
import { type SipAccountView, resolveActiveAccount } from './types'

const props = defineProps<{
  resourceId?: string
  environmentId?: string
  name?: string
}>()

const emit = defineEmits<{ 'update:status': [status: string] }>()

const { t } = useI18n()

const registered = ref(false)
const registrationFailed = ref<string | null>(null)
const incoming = ref<{ callId: string; from: string } | null>(null)
const currentCall = ref<{ callId: string; state: SipCallState; from?: string } | null>(null)
const connected = ref(false)
const micOn = ref(false)
const audio = new SipAudio()

// 浏览器实时视频（子任务 #1）：下行 RGBA 像素帧渲染到 canvas + 上行摄像头采集回传。
const video = new SipVideo()
const videoOn = ref(false)
const videoError = ref<string | null>(null)
const videoCanvas = ref<HTMLCanvasElement | null>(null)

// 实时媒体质量指标（子任务 #5）：丢帧率 / 抖动 / 延迟代理。
const quality = ref<{ loss: number; jitter: number; rtt: number } | null>(null)

let client: SipClient | null = null

// 信令抓包（子任务 #3）：UA₁ 真实 SIP 字节经 baresip 钩子全局捕获，UA₂ 中继层 JSON。
const capturing = ref(false)
const captureError = ref<string | null>(null)

async function onToggleCapture() {
  captureError.value = null
  try {
    if (!capturing.value) {
      await sipCaptureApi.start(props.resourceId || '')
      capturing.value = true
    } else {
      const r = await sipCaptureApi.stop(props.resourceId || '')
      capturing.value = false
      void r
    }
  } catch (e) {
    captureError.value = e instanceof Error ? e.message : String(e)
  }
}

// 通话录音（子任务 #2）：Hub 在通话进行时捕获下行 PCM 落盘为 WAV，关联各 CDR。
const recording = ref(false)
const recordError = ref<string | null>(null)

async function onToggleRecord() {
  recordError.value = null
  try {
    if (!recording.value) {
      await recordingApi.start(props.resourceId || '')
      recording.value = true
    } else {
      await recordingApi.stop(props.resourceId || '')
      recording.value = false
    }
  } catch (e) {
    recordError.value = e instanceof Error ? e.message : String(e)
  }
}

// 浏览器实时视频（子任务 #1）：开启后把下行像素帧渲染到 canvas，并上行采集摄像头回传。
async function onToggleVideo() {
  videoError.value = null
  try {
    if (!videoOn.value) {
      if (videoCanvas.value) video.attachCanvas(videoCanvas.value)
      await video.startCamera((f) => {
        const buf = encodeVideoFrame(f.width, f.height, f.rgba)
        client?.sendVideoFrame(buf)
      })
      videoOn.value = video.camActive
    } else {
      video.stopCamera()
      videoOn.value = false
    }
  } catch (e) {
    videoError.value = e instanceof Error ? e.message : String(e)
  }
}

// 下行视频帧（原始 RGBA 像素）→ 解码 → 渲染到 canvas。
function handleVideo(data: ArrayBuffer) {
  try {
    const { width, height, rgba } = decodeVideoFrame(data)
    video.renderFrame(width, height, rgba)
  } catch {
    // 畸形视频帧静默忽略（与音频解码同策略）。
  }
}

function getToken(): string {
  return localStorage.getItem('rex-token') || ''
}

function handleEvent(e: SipServerEvent) {
  switch (e.type) {
    case 'sip.registered':
      registered.value = true
      registrationFailed.value = null
      emit('update:status', 'online')
      break
    case 'sip.registration_failed':
      registrationFailed.value = e.payload.reason
      emit('update:status', 'error')
      break
    case 'sip.incoming':
      // 如果当前已有通话，保留当前通话，新的来电作为排队（简化：覆盖提示）
      incoming.value = { callId: e.payload.callId, from: e.payload.from }
      emit('update:status', 'connecting')
      break
    case 'sip.call_state':
      applyCallState(e.payload.callId, e.payload.state)
      break
    case 'sip.error':
      registrationFailed.value = e.payload.message
      emit('update:status', 'error')
      break
    case 'sip.sip_message':
    case 'sip.ping':
      break
    case 'sip.quality':
      quality.value = e.payload
      break
  }
}

// 下行媒体帧（原始 S16LE PCM）→ 解码 → 播放。
function handleMedia(data: ArrayBuffer) {
  const pcm = decodeMediaFrame(data)
  if (pcm.length > 0) audio.playPcm(pcm)
}

function applyCallState(callId: string, state: SipCallState) {
  if (state === 'ended') {
    if (incoming.value?.callId === callId) incoming.value = null
    if (currentCall.value?.callId === callId) currentCall.value = null
    quality.value = null
    teardownAudio()
    video.stopCamera()
    videoOn.value = false
    emit('update:status', registered.value ? 'online' : 'error')
    return
  }
  const from = currentCall.value?.from ?? incoming.value?.from
  const wasActive = currentCall.value?.state === 'active'
  currentCall.value = { callId, state, from }
  incoming.value = null
  // 进入通话（含通话中状态切换）→ 确保下行播放链路就绪。
  if (!wasActive && (state === 'active' || state === 'ringing')) {
    audio.initPlayback()
  }
  emit('update:status', state === 'ringing' ? 'connecting' : 'online')
}

async function onToggleMic() {
  if (!micOn.value) {
    audio.initPlayback()
    await audio.startMic((frame) => {
      client?.sendMediaFrame(encodePcmFrame(frame))
    })
    micOn.value = audio.micActive
  } else {
    audio.stopMic()
    micOn.value = false
  }
}

function teardownAudio() {
  audio.stopMic()
  micOn.value = false
  audio.close()
}

function onAnswer(callId: string) {
  client?.answer(callId)
  incoming.value = null
}

function onHangup(callId: string) {
  client?.hangup(callId)
  if (incoming.value?.callId === callId) incoming.value = null
}

function onHold(callId: string) {
  client?.hold(callId)
}

function onUnhold(callId: string) {
  client?.unhold(callId)
}

function onDtmf(callId: string, digit: string) {
  client?.dtmf(callId, digit)
}

function onDial(destination: string) {
  client?.dial(destination)
}

onMounted(() => {
  client = new SipClient(props.resourceId || '', {
    onEvent: handleEvent,
    onMedia: handleMedia,
    onVideo: handleVideo,
    onOpen: () => {
      connected.value = true
      emit('update:status', 'connecting')
    },
    onClose: () => {
      connected.value = false
      emit('update:status', 'disconnected')
    },
    onError: () => emit('update:status', 'error'),
  })
  client.connect(getToken())
  void loadSipResource()
})

onBeforeUnmount(() => {
  teardownAudio()
  video.stopCamera()
  video.close()
  videoOn.value = false
  client?.close()
  client = null
  capturing.value = false
})

const statusLabel = computed(() => {
  if (registrationFailed.value) return t('sip.regFailed')
  if (!connected.value) return t('sip.disconnected')
  if (registered.value) return t('sip.registered')
  return t('sip.registering')
})

// 多账户切换（0.70.4）：一个名称下挂多个账户，下拉切换生效账户并写回资源。
const sipAccounts = ref<SipAccountView[]>([])
const activeAccount = ref<string>('')
const switchingAccount = ref(false)

function parseSipProfile(raw: string) {
  try {
    const cfg = JSON.parse(raw) as {
      accounts?: SipAccountView[]
      activeAccount?: string
    }
    const accounts = cfg.accounts ?? []
    sipAccounts.value = accounts.map((a) => ({
      id: a.id,
      server: a.server,
      port: a.port,
      transport: a.transport,
      username: a.username,
      password: a.password,
      displayName: a.displayName,
    }))
    activeAccount.value = resolveActiveAccount(accounts, cfg.activeAccount ?? '')?.id ?? ''
  } catch {
    sipAccounts.value = []
    activeAccount.value = ''
  }
}

async function loadSipResource() {
  if (!props.environmentId || !props.resourceId) return
  try {
    const res: Resource = await resourcesApi.get(props.environmentId, props.resourceId)
    parseSipProfile(res.config_json)
  } catch {
    // 资源读取失败不阻断通话面板，仅不显示账户切换。
  }
}

async function selectAccount(id: string) {
  if (!props.environmentId || !props.resourceId) return
  if (id === activeAccount.value) return
  switchingAccount.value = true
  try {
    // 专用端点仅切换生效账户，后端读全量→改 activeAccount→写回，前端不发多余 GET。
    await resourcesApi.setActiveAccount(props.environmentId, props.resourceId, id)
    activeAccount.value = id
  } catch (e) {
    registrationFailed.value = e instanceof Error ? e.message : String(e)
  } finally {
    switchingAccount.value = false
  }
}
</script>

<template>
  <div class="sip-page">
    <header class="sip-header">
      <span class="sip-title">{{ name || t('sip.title') }}</span>
      <div class="sip-header-right">
        <select
          v-if="sipAccounts.length > 1"
          class="account-select"
          :value="activeAccount"
          :disabled="switchingAccount"
          @change="selectAccount(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="a in sipAccounts" :key="a.id" :value="a.id">
            {{ a.displayName || a.username }}
          </option>
        </select>
        <span v-else-if="sipAccounts.length === 1" class="sip-account-muted muted">
          {{ sipAccounts[0]?.displayName || sipAccounts[0]?.username }}
        </span>
        <span class="sip-status muted">{{ statusLabel }}</span>
      </div>
    </header>
    <div class="sip-body">
      <CallState
        :registered="registered"
        :registration-failed="registrationFailed"
        :incoming="incoming"
        :call="currentCall"
        :mic-on="micOn"
        @answer="onAnswer"
        @hangup="onHangup"
        @hold="onHold"
        @unhold="onUnhold"
        @dtmf="onDtmf"
        @toggle-mic="onToggleMic"
      />
      <Dialpad :registered="registered" @dial="onDial" />
      <div class="capture-bar">
        <button class="capture-btn" :class="{ active: capturing }" @click="onToggleCapture">
          {{ capturing ? t('sip.captureStop') : t('sip.captureStart') }}
        </button>
        <a
          v-if="capturing"
          class="capture-dl"
          :href="sipCaptureApi.pcapUrl(props.resourceId || '')"
          target="_blank"
          rel="noopener"
        >{{ t('sip.captureDownload') }}</a>
        <button class="capture-btn" :class="{ active: recording }" @click="onToggleRecord">
          {{ recording ? t('sip.recordStop') : t('sip.recordStart') }}
        </button>
        <button class="capture-btn" :class="{ active: videoOn }" @click="onToggleVideo">
          {{ videoOn ? t('sip.videoStop') : t('sip.videoStart') }}
        </button>
        <span v-if="captureError" class="capture-err muted">{{ captureError }}</span>
        <span v-if="recordError" class="capture-err muted">{{ recordError }}</span>
        <span v-if="videoError" class="capture-err muted">{{ videoError }}</span>
      </div>
      <!-- 浏览器实时视频渲染画布（子任务 #1）：下行 RGBA 像素帧经 SipVideo 渲染。 -->
      <canvas
        v-show="videoOn"
        ref="videoCanvas"
        class="video-canvas"
        width="320"
        height="240"
      ></canvas>
      <div v-if="quality && currentCall?.state === 'active'" class="quality-card">
        <div class="q-metric">
          <span class="q-label muted">{{ t('sip.qualityLoss') }}</span>
          <span class="q-value mono">{{ (quality.loss * 100).toFixed(1) }}%</span>
        </div>
        <div class="q-metric">
          <span class="q-label muted">{{ t('sip.qualityJitter') }}</span>
          <span class="q-value mono">{{ quality.jitter.toFixed(1) }} ms</span>
        </div>
        <div class="q-metric">
          <span class="q-label muted">{{ t('sip.qualityRtt') }}</span>
          <span class="q-value mono">{{ quality.rtt.toFixed(1) }} ms</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sip-page {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-deep);
  overflow: auto;
}
.sip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--statusbar-height);
  padding: 0 var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-sm);
}
.sip-header-right {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.sip-title {
  font-weight: 600;
  color: var(--text-primary);
}
.sip-status {
  font-size: var(--text-xs);
}
.account-select {
  height: 24px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-size: var(--text-xs);
  padding: 0 var(--space-1);
}
.sip-account-muted {
  font-size: var(--text-xs);
}
.sip-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) 0;
}
.capture-bar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.capture-btn {
  padding: var(--space-1) var(--space-3);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  background: var(--bg-deep);
  color: var(--text-primary);
  cursor: pointer;
  font-size: var(--text-sm);
}
.capture-btn.active {
  border-color: var(--danger, #e5484d);
  color: var(--danger, #e5484d);
}
.capture-dl {
  font-size: var(--text-sm);
  color: var(--accent, #4a9eff);
  text-decoration: none;
}
.capture-err { font-size: var(--text-xs); }
.video-canvas {
  width: 100%;
  max-height: 320px;
  background: #000;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  object-fit: contain;
}
.quality-card {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-3);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.q-metric { display: flex; flex-direction: column; gap: 2px; }
.q-label { font-size: var(--text-xs); }
.q-value { font-size: var(--text-base); font-weight: 600; color: var(--text-primary); }
.mono { font-family: var(--font-mono); }
</style>
