<template>
  <div class="resource-picker" ref="pickerRef">
    <button
      class="resource-picker-trigger"
      :class="{ open: isOpen }"
      @click="toggleOpen"
      type="button"
    >
      <span v-if="selectedResource" class="resource-picker-selected">
        <span class="resource-protocol-icon">{{ getProtocolIcon(selectedResource.protocol) }}</span>
        <span class="resource-name">{{ selectedResource.name }}</span>
        <span class="resource-address">{{ getResourceAddress(selectedResource) }}</span>
      </span>
      <span v-else class="resource-picker-placeholder">
        {{ t('notebooks.editor.command.selectResource') }}
      </span>
      <span class="resource-picker-arrow" :class="{ open: isOpen }">▾</span>
    </button>

    <Teleport to="body">
      <div
        v-if="isOpen"
        class="resource-picker-dropdown"
        :style="dropdownStyle"
        @click.stop
      >
        <div class="resource-picker-search">
          <input
            ref="searchInput"
            v-model="searchQuery"
            type="text"
            class="resource-search-input"
            :placeholder="t('notebooks.editor.command.searchPlaceholder')"
            @keydown.escape="close"
          />
        </div>
        <div class="resource-picker-list">
          <template v-if="filteredGroups.length > 0">
            <div
              v-for="group in filteredGroups"
              :key="group.id"
              class="resource-group"
            >
              <div class="resource-group-header">{{ group.name }}</div>
              <button
                v-for="resource in group.resources"
                :key="resource.id"
                class="resource-item"
                :class="{ selected: resource.id === modelValue }"
                @click="selectResource(resource)"
                type="button"
              >
                <span class="resource-protocol-icon">{{ getProtocolIcon(resource.protocol) }}</span>
                <span class="resource-item-name">{{ resource.name }}</span>
                <span class="resource-item-address">{{ getResourceAddress(resource) }}</span>
                <span class="resource-item-protocol">{{ resource.protocol }}</span>
              </button>
            </div>
          </template>
          <div v-else class="resource-picker-empty">
            {{ t('notebooks.editor.command.noResults') }}
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { listEnvsWithResources } from '@/api/env'
import type { EnvWithResources, Resource } from '@/api/env'
import { PROTOCOL_ICONS } from '@/utils/protocols'

const props = defineProps<{
  modelValue: string | null
  protocol: string | null
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | null]
  'update:protocol': [protocol: string | null]
}>()

const { t } = useI18n()
const pickerRef = ref<HTMLElement>()
const searchInput = ref<HTMLInputElement>()
const isOpen = ref(false)
const searchQuery = ref('')
const groups = ref<EnvWithResources[]>([])
const dropdownStyle = ref<Record<string, string>>({})

const selectedResource = computed(() => {
  if (!props.modelValue) return null
  for (const group of groups.value) {
    const found = group.resources.find(r => r.id === props.modelValue)
    if (found) return found
  }
  return null
})

const filteredGroups = computed(() => {
  const query = searchQuery.value.toLowerCase().trim()
  if (!query) return groups.value
  return groups.value
    .map(group => ({
      ...group,
      resources: group.resources.filter(
        r =>
          r.name.toLowerCase().includes(query) ||
          r.protocol.toLowerCase().includes(query) ||
          getResourceAddress(r).toLowerCase().includes(query)
      ),
    }))
    .filter(group => group.resources.length > 0)
})

function getProtocolIcon(protocol: string): string {
  return PROTOCOL_ICONS[protocol.toLowerCase()] ?? '⚡'
}

function getResourceAddress(resource: Resource): string {
  try {
    const cfg = JSON.parse(resource.config_json)
    if (resource.protocol === 'sqlite') return cfg.db_path || ''
    if (resource.protocol === 's3') return cfg.endpoint || ''
    if (cfg.host) return cfg.port ? `${cfg.host}:${cfg.port}` : cfg.host
    return ''
  } catch {
    return ''
  }
}

async function fetchResources() {
  try {
    groups.value = await listEnvsWithResources()
  } catch {
    groups.value = []
  }
}

function updateDropdownPosition() {
  if (!pickerRef.value) return
  const rect = pickerRef.value.getBoundingClientRect()
  dropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    minWidth: `${rect.width}px`,
    maxWidth: '400px',
    zIndex: '9999',
  }
}

function toggleOpen() {
  if (isOpen.value) {
    close()
  } else {
    open()
  }
}

async function open() {
  isOpen.value = true
  updateDropdownPosition()
  if (groups.value.length === 0) {
    fetchResources()
  }
  await nextTick()
  searchInput.value?.focus()
}

function close() {
  isOpen.value = false
  searchQuery.value = ''
}

function selectResource(resource: Resource) {
  emit('update:modelValue', resource.id)
  emit('update:protocol', resource.protocol)
  close()
}

function handleClickOutside(e: MouseEvent) {
  if (!pickerRef.value?.contains(e.target as Node)) {
    close()
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside, true)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside, true)
})

watch(isOpen, (open) => {
  if (open) {
    nextTick(() => updateDropdownPosition())
  }
})
</script>

<style scoped>
.resource-picker {
  position: relative;
  width: 100%;
}

.resource-picker-trigger {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  width: 100%;
  padding: var(--sp-sm) var(--sp-md);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  cursor: pointer;
  font-size: var(--fs-sm);
  text-align: left;
  transition: border-color var(--transition-fast);
}

.resource-picker-trigger:hover {
  border-color: var(--accent);
}

.resource-picker-trigger.open {
  border-color: var(--accent);
}

.resource-picker-selected {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  flex: 1;
  min-width: 0;
}

.resource-picker-placeholder {
  flex: 1;
  color: var(--text-muted);
  font-style: italic;
}

.resource-picker-arrow {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  transition: transform var(--transition-fast);
}

.resource-picker-arrow.open {
  transform: rotate(180deg);
}

.resource-protocol-icon {
  font-size: var(--fs-md);
  flex-shrink: 0;
}

.resource-name {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resource-address {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-left: auto;
}

.resource-picker-dropdown {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  max-height: 320px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.resource-picker-search {
  padding: var(--sp-sm);
  border-bottom: 1px solid var(--border);
}

.resource-search-input {
  width: 100%;
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--fs-sm);
  outline: none;
}

.resource-search-input:focus {
  border-color: var(--accent);
}

.resource-picker-list {
  overflow-y: auto;
  max-height: 280px;
}

.resource-group {
  padding: var(--sp-xs) 0;
}

.resource-group-header {
  padding: var(--sp-xs) var(--sp-md);
  font-size: var(--fs-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.resource-item {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  width: 100%;
  padding: var(--sp-xs) var(--sp-md);
  border: none;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  font-size: var(--fs-sm);
  text-align: left;
  transition: background-color var(--transition-fast);
}

.resource-item:hover {
  background: var(--bg-surface);
}

.resource-item.selected {
  background: var(--accent-bg, rgba(var(--accent-rgb, 59, 130, 246), 0.1));
}

.resource-item-name {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resource-item-address {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.resource-item-protocol {
  font-size: var(--fs-xs);
  color: var(--text-muted);
  font-family: var(--font-mono);
  padding: 1px 6px;
  background: var(--bg-surface);
  border-radius: var(--radius-sm);
  flex-shrink: 0;
}

.resource-picker-empty {
  padding: var(--sp-lg);
  text-align: center;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}
</style>
