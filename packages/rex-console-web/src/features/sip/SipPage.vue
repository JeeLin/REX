<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Dialpad from './Dialpad.vue'
import CallState from './CallState.vue'
import { SipClient, type SipCallState, type SipServerEvent } from '@/api/sip'

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
  }
}

function applyCallState(callId: string, state: SipCallState) {
  if (state === 'ended') {
    if (incoming.value?.callId === callId) incoming.value = null
    if (currentCall.value?.callId === callId) currentCall.value = null
    emit('update:status', registered.value ? 'online' : 'error')
    return
  }
  const from = currentCall.value?.from ?? incoming.value?.from
  currentCall.value = { callId, state, from }
  incoming.value = null
  emit('update:status', state === 'ringing' ? 'connecting' : 'online')
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
        @answer="onAnswer"
        @hangup="onHangup"
        @hold="onHold"
        @unhold="onUnhold"
        @dtmf="onDtmf"
      />
      <Dialpad :registered="registered" @dial="onDial" />
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
</style>
