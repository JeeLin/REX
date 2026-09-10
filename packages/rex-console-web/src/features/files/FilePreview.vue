<script setup lang="ts">
import { ref, watch, computed, onBeforeUnmount } from 'vue'
import * as filesApi from '@/api/files'
import Button from '@/components/ui/Button.vue'


const props = defineProps<{
  show: boolean
  file: { name: string; path: string; mime?: string } | null
  sessionId: string
}>()

const emit = defineEmits<{
  close: []
}>()

// Image preview
const imageUrl = ref('')
const imageZoom = ref(1)

// Text preview
const textContent = ref('')
const textLines = ref<string[]>([])
const textLoading = ref(false)

const IMAGE_EXTS = /\.(png|jpe?g|gif|webp|bmp|svg|ico)(\?|$)/i
const TEXT_EXTS = /\.(txt|md|json|js|ts|tsx|jsx|vue|css|scss|less|html|xml|yaml|yml|toml|ini|cfg|conf|sh|bash|zsh|py|rb|go|rs|java|c|cpp|h|hpp|sql|log|csv|env|makefile|dockerfile|docker-compose)(\?|$)/i

const MAX_TEXT_LINES = 10000

const isImage = computed(() => {
  if (!props.file) return false
  if (props.file.mime?.startsWith('image/')) return true
  return IMAGE_EXTS.test(props.file.name)
})

const isText = computed(() => {
  if (!props.file) return false
  if (props.file.mime?.startsWith('text/')) return true
  if (props.file.mime === 'application/json' || props.file.mime === 'application/javascript') return true
  return TEXT_EXTS.test(props.file.name)
})

const previewable = computed(() => isImage.value || isText.value)

watch(() => props.show, async (visible) => {
  if (visible && props.file && props.sessionId) {
    await loadPreview()
  } else {
    cleanup()
  }
})

watch(() => props.file, async () => {
  if (props.show && props.file && props.sessionId) {
    await loadPreview()
  }
})

async function loadPreview() {
  if (!props.file || !props.sessionId) return
  cleanup()
  if (isImage.value) {
    try {
      const blob = await filesApi.downloadFile(props.sessionId, props.file.path)
      imageUrl.value = URL.createObjectURL(blob)
    } catch (e) {
      console.error('Failed to load image preview:', e)
    }
  } else if (isText.value) {
    textLoading.value = true
    try {
      const result = await filesApi.readForEdit(props.sessionId, props.file.path)
      textContent.value = result.content
      textLines.value = result.content.split('\n').slice(0, MAX_TEXT_LINES)
    } catch (e) {
      console.error('Failed to load text preview:', e)
      textLines.value = [`Failed to load: ${e instanceof Error ? e.message : String(e)}`]
    } finally {
      textLoading.value = false
    }
  }
}

function cleanup() {
  if (imageUrl.value) {
    URL.revokeObjectURL(imageUrl.value)
    imageUrl.value = ''
  }
  textContent.value = ''
  textLines.value = []
  imageZoom.value = 1
}

function zoomIn() { imageZoom.value = Math.min(5, imageZoom.value + 0.25) }
function zoomOut() { imageZoom.value = Math.max(0.25, imageZoom.value - 0.25) }
function zoomReset() { imageZoom.value = 1 }

function onClose() {
  cleanup()
  emit('close')
}

function onOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget) onClose()
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') onClose()
}

// Watch for show to add/remove key listener
watch(() => props.show, (visible) => {
  if (visible) {
    document.addEventListener('keydown', onKeyDown)
  } else {
    document.removeEventListener('keydown', onKeyDown)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeyDown)
  cleanup()
})
</script>

<template>
  <Teleport to="body">
    <Transition name="fpv">
      <div v-if="show && file && previewable" class="fpv-overlay" @click="onOverlayClick">
        <div class="fpv-container">
          <!-- Header -->
          <div class="fpv-header">
            <span class="fpv-title mono">{{ file.name }}</span>
            <div class="fpv-actions">
              <!-- Image zoom controls -->
              <template v-if="isImage">
                <Button variant="ghost" icon size="sm" title="-" @click="zoomOut">−</Button>
                <span class="fpv-zoom mono">{{ Math.round(imageZoom * 100) }}%</span>
                <Button variant="ghost" icon size="sm" title="+" @click="zoomIn">+</Button>
                <Button variant="ghost" icon size="sm" title="Reset" @click="zoomReset">⊙</Button>
              </template>
              <Button variant="ghost" icon size="sm" title="Close" @click="onClose">✕</Button>
            </div>
          </div>

          <!-- Image content -->
          <div v-if="isImage" class="fpv-body fpv-body--image">
            <img
              v-if="imageUrl"
              :src="imageUrl"
              :style="{ transform: `scale(${imageZoom})` }"
              class="fpv-image"
              draggable="false"
            />
            <div v-else class="fpv-loading">Loading...</div>
          </div>

          <!-- Text content -->
          <div v-else-if="isText" class="fpv-body fpv-body--text">
            <template v-if="textLoading">
              <div class="fpv-loading">Loading...</div>
            </template>
            <template v-else>
              <pre class="fpv-text mono"><template v-for="(line, i) in textLines" :key="i">{{ line }}
</template></pre>
              <div v-if="textContent.split('\n').length > MAX_TEXT_LINES" class="fpv-truncated">
                Truncated at {{ MAX_TEXT_LINES }} lines
              </div>
            </template>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fpv-overlay {
  position: fixed;
  inset: 0;
  z-index: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.8);
}

.fpv-container {
  display: flex;
  flex-direction: column;
  width: 90vw;
  height: 90vh;
  max-width: 1200px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.fpv-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
  background: var(--bg-surface);
  flex-shrink: 0;
}

.fpv-title {
  font-size: var(--text-sm);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.fpv-actions {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-shrink: 0;
}

.fpv-zoom {
  font-size: var(--text-xs);
  color: var(--text-muted);
  min-width: 40px;
  text-align: center;
}

.fpv-body {
  flex: 1;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 0;
}

.fpv-body--image {
  background: var(--bg-deep);
  overflow: auto;
}

.fpv-body--text {
  flex-direction: column;
  overflow: auto;
  align-items: stretch;
  justify-content: flex-start;
}

.fpv-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  transition: transform 0.15s ease;
}

.fpv-text {
  margin: 0;
  padding: var(--space-3);
  font-size: var(--text-xs);
  line-height: 1.5;
  color: var(--text-primary);
  white-space: pre;
  overflow: visible;
}

.fpv-loading {
  color: var(--text-muted);
  font-size: var(--text-sm);
  padding: var(--space-4);
}

.fpv-truncated {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-align: center;
  border-top: 1px solid var(--border);
}

/* Transition */
.fpv-enter-active,
.fpv-leave-active {
  transition: opacity 0.2s ease;
}
.fpv-enter-from,
.fpv-leave-to {
  opacity: 0;
}
</style>
