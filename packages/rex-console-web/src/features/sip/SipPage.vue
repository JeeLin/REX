<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Dialpad from './Dialpad.vue'
import CallState from './CallState.vue'
import { SipClient, type SipCallState, type SipServerEvent } from '@/api/sip'
import { SipAudio, encodePcmFrame, decodeMediaFrame } from '@/api/sipMedia'

const props = defineProps<{
  resourceId?: string
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

// 实时媒体质量指标（子任务 #5）：丢帧率 / 抖动 / 延迟代理。
const quality = ref<{ loss: number; jitter: number; rtt: number } | null>(null)

let client: SipClient | null = null

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
})

onBeforeUnmount(() => {
  teardownAudio()
  client?.close()
  client = null
})

const statusLabel = computed(() => {
  if (registrationFailed.value) return t('sip.regFailed')
  if (!connected.value) return t('sip.disconnected')
  if (registered.value) return t('sip.registered')
  return t('sip.registering')
})
</script>

<template>
  <div class="sip-page">
    <header class="sip-header">
      <span class="sip-title">{{ name || t('wizard.sipServer') }}</span>
      <span class="sip-status muted">{{ statusLabel }}</span>
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
.sip-title {
  font-weight: 600;
  color: var(--text-primary);
}
.sip-status {
  font-size: var(--text-xs);
}
.sip-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) 0;
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
