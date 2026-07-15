<script setup lang="ts">
import { ref } from 'vue'
import Select from '@/components/ui/Select.vue'
import Input from '@/components/ui/Input.vue'
import Button from '@/components/ui/Button.vue'

const emit = defineEmits<{ connect: [config: { protocol: string; host: string; port: string; user: string }] }>()

const protocol = ref('ssh')
const host = ref('')
const port = ref('')
const user = ref('')

const protocols = [
  { label: 'SSH', value: 'ssh' },
  { label: 'MySQL', value: 'mysql' },
  { label: 'Redis', value: 'redis' },
  { label: 'PostgreSQL', value: 'postgresql' },
  { label: 'SFTP', value: 'sftp' },
]

function onConnect() {
  if (!host.value) return
  emit('connect', {
    protocol: protocol.value,
    host: host.value,
    port: port.value,
    user: user.value,
  })
}
</script>

<template>
  <div class="quick-connect">
    <Select v-model="protocol" :options="protocols" size="sm" />
    <Input v-model="host" placeholder="Host" size="sm" class="qc-host" />
    <Input v-model="port" placeholder="Port" size="sm" class="qc-port" />
    <Input v-model="user" placeholder="User" size="sm" class="qc-user" />
    <Button variant="primary" size="sm" :disabled="!host" @click="onConnect">Connect</Button>
  </div>
</template>

<style scoped>
.quick-connect {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  height: 36px;
}
.qc-host { width: 160px; }
.qc-port { width: 70px; }
.qc-user { width: 100px; }
</style>
