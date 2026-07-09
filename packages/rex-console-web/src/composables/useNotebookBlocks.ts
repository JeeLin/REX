import { ref, computed, onUnmounted, type Ref, type ComputedRef } from 'vue'
import type { NotebookBlock, EditorBlock, NotebookWithBlocks } from '@/api/notebook'
import { updateBlocks } from '@/api/notebook'

let idCounter = 0
function generateTempId() {
  return `_temp_${Date.now()}_${++idCounter}`
}

function apiBlockToEditorBlock(block: NotebookBlock): EditorBlock {
  const type = block.block_type as EditorBlock['type']
  return {
    id: block.id,
    type,
    content: block.content,
    level: type === 'heading' ? parseHeadingLevel(block.content) : undefined,
    resourceId: block.resource_id ?? undefined,
    protocol: block.protocol ?? undefined,
    orderIndex: block.order_index,
  }
}

function parseHeadingLevel(content: string): number {
  const match = content.match(/^(#{1,3})/)
  return match ? match[1]!.length : 1
}

function editorBlockToApiPayload(block: EditorBlock, isNew: boolean) {
  const payload: {
    id?: string
    block_type: string
    content?: string
    resource_id?: string
    protocol?: string
    order_index: number
  } = {
    block_type: block.type,
    content: block.content,
    order_index: block.orderIndex,
  }
  if (!isNew) {
    payload.id = block.id
  }
  if (block.resourceId) {
    payload.resource_id = block.resourceId
  }
  if (block.protocol) {
    payload.protocol = block.protocol
  }
  return payload
}

export interface NotebookBlocksState {
  blocks: Ref<EditorBlock[]>
  focusedBlockId: Ref<string | null>
  focusedBlock: ComputedRef<EditorBlock | undefined>
  focusedBlockIndex: ComputedRef<number>
  initializeBlocks: (notebook: NotebookWithBlocks) => void
  addBlock: (afterId: string | null, type?: EditorBlock['type']) => EditorBlock
  removeBlock: (id: string) => void
  updateBlockContent: (id: string, content: string) => void
  updateBlockResource: (id: string, resourceId: string, protocol: string) => void
  setFocusedBlock: (id: string | null) => void
  adjustHeadingLevel: (id: string, direction: 1 | -1) => void
  focusPreviousBlock: () => void
  focusNextBlock: () => void
  saveBlocks: () => Promise<void>
}

export function useNotebookBlocks(
  notebookId: string,
  onSaved?: () => void
): NotebookBlocksState {
  const blocks = ref<EditorBlock[]>([])
  const focusedBlockId = ref<string | null>(null)
  let isDirty = false
  let pendingSave: ReturnType<typeof setTimeout> | null = null

  const focusedBlock = computed(() =>
    blocks.value.find((b) => b.id === focusedBlockId.value)
  )

  const focusedBlockIndex = computed(() =>
    blocks.value.findIndex((b) => b.id === focusedBlockId.value)
  )

  function initializeBlocks(notebook: NotebookWithBlocks) {
    if (notebook.blocks.length === 0) {
      const initial: EditorBlock = {
        id: generateTempId(),
        type: 'paragraph',
        content: '',
        orderIndex: 0,
      }
      blocks.value = [initial]
    } else {
      blocks.value = notebook.blocks
        .slice()
        .sort((a, b) => a.order_index - b.order_index)
        .map(apiBlockToEditorBlock)
    }
  }

  function addBlock(afterId: string | null, type: EditorBlock['type'] = 'paragraph'): EditorBlock {
    const afterIdx = afterId
      ? blocks.value.findIndex((b) => b.id === afterId)
      : blocks.value.length - 1
    const newIdx = afterIdx + 1
    const newBlock: EditorBlock = {
      id: generateTempId(),
      type,
      content: '',
      level: type === 'heading' ? 1 : undefined,
      orderIndex: newIdx,
    }
    blocks.value.splice(newIdx, 0, newBlock)
    reindex()
    markDirty()
    return newBlock
  }

  function removeBlock(id: string) {
    const idx = blocks.value.findIndex((b) => b.id === id)
    if (idx === -1) return
    blocks.value.splice(idx, 1)
    if (blocks.value.length === 0) {
      blocks.value.push({
        id: generateTempId(),
        type: 'paragraph',
        content: '',
        orderIndex: 0,
      })
    } else {
      reindex()
    }
    markDirty()
  }

  function updateBlockContent(id: string, content: string) {
    const block = blocks.value.find((b) => b.id === id)
    if (block) {
      block.content = content
      markDirty()
    }
  }
  function updateBlockResource(id: string, resourceId: string, protocol: string) {
    const block = blocks.value.find((b) => b.id === id)
    if (block) {
      block.resourceId = resourceId
      block.protocol = protocol
      markDirty()
    }
  }

  function setFocusedBlock(id: string | null) {
    focusedBlockId.value = id
  }

  function adjustHeadingLevel(id: string, direction: 1 | -1) {
    const block = blocks.value.find((b) => b.id === id)
    if (block && block.type === 'heading') {
      const current = block.level ?? 1
      block.level = Math.max(1, Math.min(3, current + direction))
      markDirty()
    }
  }

  function focusPreviousBlock() {
    const idx = focusedBlockIndex.value
    if (idx > 0) {
      focusedBlockId.value = blocks.value[idx - 1]!.id
    }
  }

  function focusNextBlock() {
    const idx = focusedBlockIndex.value
    if (idx < blocks.value.length - 1) {
      focusedBlockId.value = blocks.value[idx + 1]!.id
    }
  }

  function reindex() {
    blocks.value.forEach((b, i) => {
      b.orderIndex = i
    })
  }

  function markDirty() {
    isDirty = true
    clearTimeout(pendingSave!)
    pendingSave = setTimeout(saveBlocks, 500)
  }

  async function saveBlocks() {
    if (!isDirty) return
    isDirty = false
    if (pendingSave) {
      clearTimeout(pendingSave)
      pendingSave = null
    }

    const seen = new Set<string>()
    const deduped = blocks.value.filter((b) => {
      if (seen.has(b.id)) return false
      seen.add(b.id)
      return true
    })
    blocks.value = deduped

    reindex()

    const apiBlocks = blocks.value.map((block) =>
      editorBlockToApiPayload(block, !block.id || block.id.startsWith('_temp_'))
    )

    try {
      await updateBlocks(notebookId, apiBlocks)
      onSaved?.()
    } catch (e) {
      console.error('Failed to save blocks:', e)
    }
  }

  onUnmounted(() => {
    if (pendingSave) {
      clearTimeout(pendingSave)
      saveBlocks()
    }
  })

  return {
    blocks,
    focusedBlockId,
    focusedBlock,
    focusedBlockIndex,
    initializeBlocks,
    addBlock,
    removeBlock,
    updateBlockContent,
    updateBlockResource,
    setFocusedBlock,
    adjustHeadingLevel,
    focusPreviousBlock,
    focusNextBlock,
    saveBlocks,
  }
}
