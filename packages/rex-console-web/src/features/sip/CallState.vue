<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SipCallState } from '@/api/sip'

const props = defineProps<{
  registered: boolean
  registrationFailed?: string | null
  incoming: { callId: string; from: string } | null
  call: { callId: string; state: SipCallState; from?: string } | null
  micOn: boolean
}>()

const emit = defineEmits<{
  answer: [callId: string]
  hangup: [callId: string]
  hold: [callId: string]
  unhold: [callId: string]
  dtmf: [callId: string, digit: string]
  toggleMic: []
}>()

const { t } = useI18n()
const dtmfKeys = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '*', '#']
</script>

<template>
  <div class="call-state">
    <!-- Registration status -->
    <div class="reg-row">
      <span
        class="status-dot"
        :class="{
          'status-dot--ok': registered,
          'status-dot--err': registrationFailed,
        }"
      />
      <span v-if="registrationFailed" class="reg-text reg-text--err">
        {{ t('sip.regFailed') }}: {{ registrationFailed }}
      </span>
      <span v-else-if="registered" class="reg-text">{{ t('sip.registered') }}</span>
      <span v-else class="reg-text muted">{{ t('sip.registering') }}</span>
    </div>

    <!-- Incoming call popup -->
    <div v-if="incoming" class="incoming">
      <div class="incoming-from">
        <span class="incoming-label">{{ t('sip.incoming') }}</span>
        <span class="incoming-number">{{ incoming.from }}</span>
      </div>
      <div class="incoming-actions">
        <button class="btn-decline" @click="emit('hangup', incoming.callId)">{{ t('sip.decline') }}</button>
        <button class="btn-answer" @click="emit('answer', incoming.callId)">{{ t('sip.answer') }}</button>
      </div>
    </div>

    <!-- Active call -->
    <div v-else-if="call" class="active-call">
      <div class="active-row">
        <span
          class="status-dot"
          :class="{
            'status-dot--ok': call.state === 'active',
            'status-dot--held': call.state === 'held',
            'status-dot--ring': call.state === 'ringing',
          }"
        />
        <span class="active-number">{{ call.from || call.callId }}</span>
        <span class="active-state muted">{{ call.state }}</span>
      </div>

      <div class="dtmf-grid">
        <button
          v-for="d in dtmfKeys"
          :key="d"
          class="dtmf-key"
          @click="emit('dtmf', call.callId, d)"
        >
          {{ d }}
        </button>
      </div>

      <div class="call-actions">
        <button
          v-if="call.state === 'held'"
          class="btn-action"
          @click="emit('unhold', call.callId)"
        >
          {{ t('sip.unhold') }}
        </button>
        <button
          v-else-if="call.state === 'active'"
          class="btn-action"
          @click="emit('hold', call.callId)"
        >
          {{ t('sip.hold') }}
        </button>
        <button
          class="btn-action"
          :class="{ 'btn-action--on': micOn }"
          :disabled="call.state !== 'active'"
          @click="emit('toggleMic')"
        >
          {{ micOn ? t('sip.micOn') : t('sip.micOff') }}
        </button>
        <button class="btn-hangup" @click="emit('hangup', call.callId)">{{ t('sip.hangup') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.call-state {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4);
}
.reg-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.reg-text {
  font-size: var(--text-sm);
}
.reg-text--err {
  color: var(--danger);
}
.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-muted);
  flex-shrink: 0;
}
.status-dot--ok {
  background: var(--success);
}
.status-dot--err {
  background: var(--danger);
}
.status-dot--held {
  background: var(--warning, #d29922);
}
.status-dot--ring {
  background: var(--accent);
}
.incoming {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: var(--bg-surface);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.incoming-from {
  display: flex;
  flex-direction: column;
}
.incoming-label {
  font-size: var(--text-xs);
  color: var(--text-muted);
}
.incoming-number {
  font-family: var(--font-mono);
  font-size: var(--text-md);
  color: var(--text-primary);
}
.incoming-actions {
  display: flex;
  gap: var(--space-2);
}
.btn-answer {
  flex: 1;
  background: var(--success);
  border: none;
  color: #fff;
  border-radius: 8px;
  padding: 10px;
  font-weight: 600;
  cursor: pointer;
}
.btn-decline {
  background: var(--bg-hover);
  border: 1px solid var(--border);
  color: var(--danger);
  border-radius: 8px;
  padding: 10px var(--space-3);
  cursor: pointer;
}
.active-call {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: var(--bg-surface);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.active-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.active-number {
  font-family: var(--font-mono);
  font-size: var(--text-md);
  color: var(--text-primary);
}
.active-state {
  font-size: var(--text-xs);
  text-transform: uppercase;
  margin-left: auto;
}
.dtmf-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 4px;
}
.dtmf-key {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  border-radius: 6px;
  height: 32px;
  cursor: pointer;
}
.dtmf-key:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}
.call-actions {
  display: flex;
  gap: var(--space-2);
}
.btn-action {
  background: var(--bg-hover);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: 8px;
  padding: 10px var(--space-3);
  cursor: pointer;
}
.btn-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-action--on {
  background: var(--success);
  border-color: var(--success);
  color: #fff;
}
.btn-hangup {
  flex: 1;
  background: var(--danger);
  border: none;
  color: #fff;
  border-radius: 8px;
  padding: 10px;
  font-weight: 600;
  cursor: pointer;
}
</style>
