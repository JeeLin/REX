<template>
  <div class="notebook-editor" @keydown="handleGlobalKeydown">
    <EditorBlock
      v-for="block in blocks"
      :key="block.id"
      :ref="(el: any) => setBlockRef(block.id, el)"
      :block="block"
      :focused="focusedBlockId === block.id"
      :suppress-keyboard="slashMenuVisible"
      @update:content="(c: string) => handleContentUpdate(block.id, c)"
      @update:resource-id="(id: string) => handleResourceUpdate(block.id, id)"
      @update:protocol="(p: string) => handleProtocolUpdate(block.id, p)"
      @enter-pressed="handleEnter(block.id)"
      @backspace-empty="handleBackspace(block.id)"
      @adjust-level="(d: 1 | -1) => adjustHeadingLevel(block.id, d)"
      @add-after="handleAddAfter(block.id)"
      @focus="setFocusedBlock(block.id)"
    />
    <SlashMenu
      :visible="slashMenuVisible"
      :position="slashMenuPosition"
      :filter="slashFilter"
      @select="handleSlashSelect"
      @close="closeSlashMenu"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { NotebookWithBlocks } from '@/api/notebook'
import type { EditorBlock } from '@/api/notebook'
import { useNotebookBlocks } from '@/composables/useNotebookBlocks'
import EditorBlockComp from './EditorBlock.vue'
import SlashMenu from './SlashMenu.vue'

const props = defineProps<{
  notebook: NotebookWithBlocks
}>()

const emit = defineEmits<{
  saved: []
}>()

const {
  blocks,
  focusedBlockId,
  initializeBlocks,
  addBlock,
  removeBlock,
  updateBlockContent,
  updateBlockResource,
  setFocusedBlock,
  adjustHeadingLevel,
  focusPreviousBlock,
  focusNextBlock,
} = useNotebookBlocks(props.notebook.id, () => emit('saved'))

// Block ref management
const blockRefs = ref<Record<string, InstanceType<typeof EditorBlockComp>>>({})

function setBlockRef(id: string, el: InstanceType<typeof EditorBlockComp> | null) {
  if (el) {
    blockRefs.value[id] = el
  } else {
    delete blockRefs.value[id]
  }
}

function focusBlock(id: string, atEnd = false) {
  setFocusedBlock(id)
  nextTick(() => {
    const el = blockRefs.value[id]
    if (el) {
      el.focus()
      if (atEnd) {
        const contentEl = el.$el?.querySelector('[contenteditable]') as HTMLElement
        if (contentEl) {
          const range = document.createRange()
          const sel = window.getSelection()
          range.selectNodeContents(contentEl)
          range.collapse(false)
          sel?.removeAllRanges()
          sel?.addRange(range)
        }
      }
    }
  })
}

// Initialize blocks when notebook loads
watch(
  () => props.notebook,
  (nb) => {
    if (nb) initializeBlocks(nb)
  },
  { immediate: true }
)

// Content updates
function handleContentUpdate(id: string, content: string) {
  updateBlockContent(id, content)
  detectSlashInput(id, content)
}
// Resource/protocol updates from CommandBlock
const pendingProtocols = new Map<string, string>()

function handleResourceUpdate(id: string, resourceId: string) {
  const block = blocks.value.find(b => b.id === id)
  if (!block) return
  const protocol = pendingProtocols.get(id) ?? block.protocol ?? ''
  pendingProtocols.delete(id)
  updateBlockResource(id, resourceId, protocol)
}

function handleProtocolUpdate(id: string, protocol: string) {
  const block = blocks.value.find(b => b.id === id)
  if (!block) return
  if (block.resourceId) {
    updateBlockResource(id, block.resourceId, protocol)
  } else {
    pendingProtocols.set(id, protocol)
  }
}

// Enter — create new paragraph block after current
function handleEnter(id: string) {
  closeSlashMenu()
  const newBlock = addBlock(id)
  focusBlock(newBlock.id)
}

// Backspace on empty block — remove it and focus previous
function handleBackspace(id: string) {
  closeSlashMenu()
  const idx = blocks.value.findIndex((b) => b.id === id)
  if (blocks.value.length <= 1) return
  const prevIdx = idx > 0 ? idx - 1 : 1
  const prevBlock = blocks.value[prevIdx]
  removeBlock(id)
  if (prevBlock) {
    focusBlock(prevBlock.id, true)
  }
}

function handleAddAfter(id: string) {
  const newBlock = addBlock(id)
  focusBlock(newBlock.id)
}


// ── Slash Menu ──────────────────────────────────────────

const slashMenuVisible = ref(false)
const slashMenuPosition = ref({ top: 0, left: 0 })
const slashFilter = ref('')
const slashBlockId = ref<string | null>(null)

function detectSlashInput(_id: string, content: string) {
  if (content === '/') {
    slashBlockId.value = _id
    positionSlashMenu()
    slashMenuVisible.value = true
    slashFilter.value = ''
  } else if (slashMenuVisible.value && _id === slashBlockId.value) {
    // Update filter from content after the slash
    const slashIdx = content.lastIndexOf('/')
    if (slashIdx >= 0) {
      slashFilter.value = content.slice(slashIdx + 1)
    } else {
      closeSlashMenu()
    }
  }
}

function positionSlashMenu() {
  const sel = window.getSelection()
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0)
    const rect = range.getBoundingClientRect()
    slashMenuPosition.value = {
      top: rect.bottom + 4,
      left: rect.left,
    }
  }
}

function handleSlashSelect(type: string) {
  if (!slashBlockId.value) return

  // Remove the "/" and any filter text from current block
  const block = blocks.value.find((b) => b.id === slashBlockId.value)
  if (block) {
    const content = block.content
    const slashIdx = content.lastIndexOf('/')
    const cleaned = slashIdx >= 0 ? content.slice(0, slashIdx) : ''
    updateBlockContent(block.id, cleaned)
  }

  // Change block type if on same block, or insert new
  if (block && block.type === 'paragraph' && !block.content.replace(/\/.*/, '').trim()) {
    // Empty paragraph — convert to new type
    block.type = type as EditorBlock['type']
    if (type === 'heading') {
      block.level = 1
    }
  } else {
    const newBlock = addBlock(slashBlockId.value, type as EditorBlock['type'])
    focusBlock(newBlock.id)
  }

  closeSlashMenu()
}

function closeSlashMenu() {
  slashMenuVisible.value = false
  slashBlockId.value = null
}

function handleGlobalKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowUp') {
    if (!slashMenuVisible.value && focusedBlockId.value) {
      e.preventDefault()
      focusPreviousBlock()
      if (focusedBlockId.value) focusBlock(focusedBlockId.value)
    }
  } else if (e.key === 'ArrowDown') {
    if (!slashMenuVisible.value && focusedBlockId.value) {
      e.preventDefault()
      focusNextBlock()
      if (focusedBlockId.value) focusBlock(focusedBlockId.value)
    }
  } else if (e.key === 'Escape') {
    closeSlashMenu()
  }
}
</script>

<style scoped>
.notebook-editor {
  width: 100%;
  min-height: 400px;
}
</style>
