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
  host: string
  port: string
  user: string
  password: string
  encoding: string
  color: string
  notes: string
}

const props = defineProps<{
  show: boolean
  resource?: ResourceProps
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
  encoding: 'UTF-8',
  color: '',
  notes: '',
})

watch(() => props.resource, (r) => {
  if (r) form.value = { ...r }
}, { immediate: true })

const activeTab = ref('connection')
const propTabs = ['connection', 'auth', 'terminal', 'notes']

const encodings = [
  { label: 'UTF-8', value: 'UTF-8' },
  { label: 'GBK', value: 'GBK' },
  { label: 'ISO-8859-1', value: 'ISO-8859-1' },
]

function onSave() {
  emit('save', { ...form.value })
  emit('update:show', false)
}
</script>

<template>
  <Modal :model-value="show" :title="`Properties — ${form.name}`" width="520px" @update:model-value="emit('update:show', $event)">
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
            <Select v-model="form.protocol" :options="[
              { label: 'SSH', value: 'ssh' },
              { label: 'MySQL', value: 'mysql' },
              { label: 'Redis', value: 'redis' },
              { label: 'PostgreSQL', value: 'postgresql' },
              { label: 'SFTP', value: 'sftp' },
            ]" size="sm" />
          </div>
          <div class="props-row">
            <div class="props-field props-field--grow">
              <label class="props-label">Host</label>
              <Input v-model="form.host" size="sm" placeholder="10.0.1.5" />
            </div>
            <div class="props-field" style="width: 100px">
              <label class="props-label">Port</label>
              <Input v-model="form.port" size="sm" placeholder="22" />
            </div>
          </div>
        </template>

        <!-- Auth -->
        <template v-if="activeTab === 'auth'">
          <div class="props-field">
            <label class="props-label">Username</label>
            <Input v-model="form.user" size="sm" placeholder="root" />
          </div>
          <div class="props-field">
            <label class="props-label">Password</label>
            <Input v-model="form.password" size="sm" placeholder="••••••" />
          </div>
        </template>

        <!-- Terminal -->
        <template v-if="activeTab === 'terminal'">
          <div class="props-field">
            <label class="props-label">Encoding</label>
            <Select v-model="form.encoding" :options="encodings" size="sm" />
          </div>
        </template>

        <!-- Notes -->
        <template v-if="activeTab === 'notes'">
          <div class="props-field">
            <label class="props-label">Notes</label>
            <textarea v-model="form.notes" class="props-textarea" rows="6" placeholder="Add notes about this resource..." />
          </div>
        </template>
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
</style>
