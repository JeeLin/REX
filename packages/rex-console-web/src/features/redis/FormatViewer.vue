<script setup lang="ts">
import { ref, computed, watch } from 'vue'

const props = defineProps<{
  value: string
}>()

type Format = 'text' | 'hex' | 'json' | 'binary'

const activeFormat = ref<Format>('text')
const formatDetected = ref<Format>('text')

// Detect format
function detectFormat(val: string): Format {
  if (!val) return 'text'
  // Try JSON
  try {
    JSON.parse(val)
    return 'json'
  } catch { /* not json */ }
  // Check for non-printable characters (binary)
  const hasBinary = /[\x00-\x08\x0e-\x1f]/.test(val)
  if (hasBinary) return 'binary'
  return 'text'
}

watch(() => props.value, (val) => {
  formatDetected.value = detectFormat(val)
  activeFormat.value = formatDetected.value
}, { immediate: true })

const displayValue = computed(() => {
  const val = props.value || ''
  switch (activeFormat.value) {
    case 'json':
      try {
        return JSON.stringify(JSON.parse(val), null, 2)
      } catch {
        return val
      }
    case 'hex':
      return Array.from(new TextEncoder().encode(val))
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ')
    case 'binary':
      return Array.from(new TextEncoder().encode(val))
        .map(b => b.toString(2).padStart(8, '0'))
        .join(' ')
    default:
      return val
  }
})

const byteSize = computed(() => {
  const bytes = new TextEncoder().encode(props.value || '')
  if (bytes.length < 1024) return `${bytes.length} bytes`
  return `${(bytes.length / 1024).toFixed(1)} KB`
})

const formatOptions: { value: Format; label: string }[] = [
  { value: 'text', label: 'Text' },
  { value: 'hex', label: 'Hex' },
  { value: 'json', label: 'JSON' },
  { value: 'binary', label: 'Binary' },
]
</script>

<template>
  <div class="format-viewer">
    <div class="format-toolbar">
      <div class="format-tabs">
        <button
          v-for="opt in formatOptions"
          :key="opt.value"
          class="format-tab"
          :class="{ 'format-tab--active': activeFormat === opt.value }"
          @click="activeFormat = opt.value"
        >
          {{ opt.label }}
        </button>
      </div>
      <span class="format-size muted">Size: {{ byteSize }}</span>
    </div>
    <pre class="format-content mono">{{ displayValue }}</pre>
  </div>
</template>

<style scoped>
.format-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.format-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-1) var(--space-2);
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
}

.format-tabs {
  display: flex;
  gap: 2px;
}

.format-tab {
  padding: var(--space-1) var(--space-2);
  background: none;
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  cursor: pointer;
  font-size: var(--text-xs);
  transition: all var(--transition);
}

.format-tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.format-tab--active {
  color: var(--accent);
  background: rgba(232, 145, 45, 0.1);
}

.format-size {
  font-size: var(--text-xs);
}

.muted {
  color: var(--text-muted);
}

.format-content {
  flex: 1;
  overflow: auto;
  padding: var(--space-3);
  margin: 0;
  background: var(--bg-deep);
  border-radius: var(--radius);
  font-size: var(--text-sm);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 400px;
}
</style>
