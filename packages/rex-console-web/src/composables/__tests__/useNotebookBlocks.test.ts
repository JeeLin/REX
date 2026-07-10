import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('../../api/client', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn().mockResolvedValue({ data: { data: [] } }),
    delete: vi.fn(),
  },
}))

vi.mock('../../router', () => ({
  default: { push: vi.fn() },
}))

import { useNotebookBlocks } from '../useNotebookBlocks'
import type { NotebookWithBlocks } from '@/api/notebook'
import { updateBlocks } from '@/api/notebook'

const mockNotebook: NotebookWithBlocks = {
  id: 'nb-1',
  title: 'Test Notebook',
  description: null,
  tags: '[]',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  blocks: [
    {
      id: 'blk-1',
      notebook_id: 'nb-1',
      block_type: 'paragraph',
      content: 'Hello',
      resource_id: null,
      protocol: null,
      order_index: 0,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    },
    {
      id: 'blk-2',
      notebook_id: 'nb-1',
      block_type: 'heading',
      content: '# Title',
      resource_id: null,
      protocol: null,
      order_index: 1,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    },
  ],
}

describe('useNotebookBlocks', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initializes blocks from notebook data', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    expect(state.blocks.value).toHaveLength(2)
    expect(state.blocks.value[0]!.id).toBe('blk-1')
    expect(state.blocks.value[0]!.content).toBe('Hello')
  })

  it('creates initial paragraph when notebook has no blocks', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks({ ...mockNotebook, blocks: [] })
    expect(state.blocks.value).toHaveLength(1)
    expect(state.blocks.value[0]!.type).toBe('paragraph')
  })

  it('adds block after specified id', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    const newBlock = state.addBlock('blk-1', 'code')
    expect(state.blocks.value).toHaveLength(3)
    expect(newBlock.type).toBe('code')
    expect(state.blocks.value[1]!.id).toBe(newBlock.id)
  })

  it('adds block at end when afterId is null', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    const newBlock = state.addBlock(null, 'paragraph')
    expect(state.blocks.value[2]!.id).toBe(newBlock.id)
  })

  it('removes block by id', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    state.removeBlock('blk-1')
    expect(state.blocks.value).toHaveLength(1)
    expect(state.blocks.value[0]!.id).toBe('blk-2')
  })

  it('does not remove last block', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks({ ...mockNotebook, blocks: [mockNotebook.blocks[0]!] })
    state.removeBlock('blk-1')
    expect(state.blocks.value).toHaveLength(1)
  })

  it('updates block content', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    state.updateBlockContent('blk-1', 'World')
    expect(state.blocks.value[0]!.content).toBe('World')
  })

  it('updates block resource and protocol', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    state.updateBlockResource('blk-1', 'res-1', 'ssh')
    expect(state.blocks.value[0]!.resourceId).toBe('res-1')
    expect(state.blocks.value[0]!.protocol).toBe('ssh')
  })

  it('sets focused block', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    state.setFocusedBlock('blk-2')
    expect(state.focusedBlockId.value).toBe('blk-2')
    expect(state.focusedBlock.value!.id).toBe('blk-2')
    expect(state.focusedBlockIndex.value).toBe(1)
  })
  it('adjusts heading level up and down', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    // blk-2 is heading with content '# Title' (level 1)
    state.adjustHeadingLevel('blk-2', 1)
    expect(state.blocks.value[1]!.level).toBe(2)
    state.adjustHeadingLevel('blk-2', 1)
    expect(state.blocks.value[1]!.level).toBe(3)
    state.adjustHeadingLevel('blk-2', -1)
    expect(state.blocks.value[1]!.level).toBe(2)
  })

  it('focuses previous and next block', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    state.setFocusedBlock('blk-2')
    state.focusPreviousBlock()
    expect(state.focusedBlockId.value).toBe('blk-1')
    state.focusNextBlock()
    expect(state.focusedBlockId.value).toBe('blk-2')
  })

  it('does not focus beyond bounds', () => {
    const state = useNotebookBlocks('nb-1')
    state.initializeBlocks(mockNotebook)
    state.setFocusedBlock('blk-1')
    state.focusPreviousBlock()
    expect(state.focusedBlockId.value).toBe('blk-1')
    state.setFocusedBlock('blk-2')
    state.focusNextBlock()
    expect(state.focusedBlockId.value).toBe('blk-2')
  })
})
