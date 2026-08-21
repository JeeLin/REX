<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { FormatInfo } from '@/api/redis'

const props = defineProps<{
  value: string
  formatInfo?: FormatInfo
}>()

type Format = 'text' | 'hex' | 'json' | 'binary' | 'msgpack' | 'php_serialize' | 'java_serialize' | 'pickle' | 'compressed'

const activeFormat = ref<Format>('text')

// Format metadata: label + optional color for tab badges
const FORMAT_META: Record<Format, { label: string; color?: string }> = {
  text: { label: 'Text' },
  hex: { label: 'Hex' },
  json: { label: 'JSON' },
  binary: { label: 'Binary' },
  msgpack: { label: 'Msgpack', color: 'var(--proto-mysql)' },
  php_serialize: { label: 'PHP', color: 'var(--purple)' },
  java_serialize: { label: 'Java', color: 'var(--warning)' },
  pickle: { label: 'Pickle', color: 'var(--success)' },
  compressed: { label: 'Compressed', color: 'var(--danger)' },
}

// Client-side format detection (fallback when no formatInfo from backend)
function detectFormat(val: string): Format {
  if (!val) return 'text'
  try { JSON.parse(val); return 'json' } catch { /* not json */ }
  if (/[\x00-\x08\x0e-\x1f]/.test(val)) return 'binary'
  return 'text'
}

// Determine active format: prefer backend formatInfo, fallback to client detection
const detectedFormat = computed<Format>(() => {
  if (props.formatInfo?.detected) {
    const f = props.formatInfo.detected
    if (f in FORMAT_META) return f as Format
  }
  return detectFormat(props.value)
})

watch(detectedFormat, (f) => { activeFormat.value = f }, { immediate: true })

// Available tabs: always show text/hex/json/binary, add detected format if different
const formatOptions = computed(() => {
  const base: { value: Format; label: string; color?: string }[] = [
    { value: 'text', label: 'Text' },
    { value: 'hex', label: 'Hex' },
    { value: 'json', label: 'JSON' },
    { value: 'binary', label: 'Binary' },
  ]
  const detected = detectedFormat.value
  const isAdvanced = !['text', 'hex', 'json', 'binary'].includes(detected)
  if (isAdvanced) {
    const meta = FORMAT_META[detected]
    base.push({ value: detected, label: meta.label, color: meta.color })
  }
  return base
})

// Display value: backend decoded for advanced formats, client-side for basic
const displayValue = computed(() => {
  const val = props.value || ''
  // For advanced formats, use backend-provided decoded content
  if (props.formatInfo?.decoded && activeFormat.value !== 'text' && activeFormat.value !== 'hex' && activeFormat.value !== 'json' && activeFormat.value !== 'binary') {
    return props.formatInfo.decoded
  }
  switch (activeFormat.value) {
    case 'json':
      try { return JSON.stringify(JSON.parse(val), null, 2) } catch { return val }
    case 'hex':
      return Array.from(new TextEncoder().encode(val))
        .map(b => b.toString(16).padStart(2, '0')).join(' ')
    case 'binary':
      return Array.from(new TextEncoder().encode(val))
        .map(b => b.toString(2).padStart(8, '0')).join(' ')
    default:
      return val
  }
})

const byteSize = computed(() => {
  const bytes = new TextEncoder().encode(props.value || '')
  if (bytes.length < 1024) return `${bytes.length} bytes`
  return `${(bytes.length / 1024).toFixed(1)} KB`
})
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
          :style="opt.color ? { '--tab-color': opt.color } as any : {}"
          @click="activeFormat = opt.value"
        >
          <span v-if="opt.color" class="format-dot" :style="{ background: opt.color }" />
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
  display: flex;
  align-items: center;
  gap: 4px;
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

.format-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
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
