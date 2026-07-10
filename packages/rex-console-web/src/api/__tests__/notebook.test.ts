import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('../client', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  },
}))

import {
  listNotebooks,
  getNotebook,
  createNotebook,
  updateNotebook,
  deleteNotebook,
  updateBlocks,
  exportNotebook,
  importNotebook,
  executeCommand,
  listExecutions,
} from '../notebook'
import client from '../client'

const mockClient = vi.mocked(client)

describe('notebook API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listNotebooks fetches all notebooks', async () => {
    const mockData = [{ id: 'nb-1', title: 'Test' }]
    mockClient.get.mockResolvedValue({ data: { data: mockData } })
    const result = await listNotebooks()
    expect(mockClient.get).toHaveBeenCalledWith('/api/notebooks')
    expect(result).toEqual(mockData)
  })

  it('getNotebook fetches a single notebook with blocks', async () => {
    const mockData = { id: 'nb-1', title: 'Test', blocks: [] }
    mockClient.get.mockResolvedValue({ data: { data: mockData } })
    const result = await getNotebook('nb-1')
    expect(mockClient.get).toHaveBeenCalledWith('/api/notebooks/nb-1')
    expect(result).toEqual(mockData)
  })

  it('createNotebook sends POST request', async () => {
    const mockData = { id: 'nb-1', title: 'New' }
    mockClient.post.mockResolvedValue({ data: { data: mockData } })
    const result = await createNotebook({ title: 'New' })
    expect(mockClient.post).toHaveBeenCalledWith('/api/notebooks', { title: 'New' })
    expect(result).toEqual(mockData)
  })

  it('updateNotebook sends PUT request', async () => {
    const mockData = { id: 'nb-1', title: 'Updated' }
    mockClient.put.mockResolvedValue({ data: { data: mockData } })
    const result = await updateNotebook('nb-1', { title: 'Updated' })
    expect(mockClient.put).toHaveBeenCalledWith('/api/notebooks/nb-1', { title: 'Updated' })
    expect(result).toEqual(mockData)
  })

  it('deleteNotebook sends DELETE request', async () => {
    mockClient.delete.mockResolvedValue({})
    await deleteNotebook('nb-1')
    expect(mockClient.delete).toHaveBeenCalledWith('/api/notebooks/nb-1')
  })

  it('updateBlocks sends PUT with blocks payload', async () => {
    const mockBlocks = [{ id: 'blk-1', block_type: 'paragraph' }]
    mockClient.put.mockResolvedValue({ data: { data: mockBlocks } })
    const result = await updateBlocks('nb-1', [
      { id: 'blk-1', block_type: 'paragraph', order_index: 0 },
    ])
    expect(mockClient.put).toHaveBeenCalledWith('/api/notebooks/nb-1/blocks', {
      blocks: [{ id: 'blk-1', block_type: 'paragraph', order_index: 0 }],
    })
    expect(result).toEqual(mockBlocks)
  })

  it('exportNotebook fetches export data', async () => {
    const mockData = { 'rex-notebook': '1.0', title: 'Test', blocks: [] }
    mockClient.get.mockResolvedValue({ data: { data: mockData } })
    const result = await exportNotebook('nb-1')
    expect(mockClient.get).toHaveBeenCalledWith('/api/notebooks/export/nb-1')
    expect(result).toEqual(mockData)
  })

  it('importNotebook sends POST with data', async () => {
    const mockData = { id: 'nb-1', title: 'Imported' }
    mockClient.post.mockResolvedValue({ data: { data: mockData } })
    const result = await importNotebook({
      title: 'Imported',
      blocks: [{ type: 'paragraph', content: 'Hi' }],
    })
    expect(mockClient.post).toHaveBeenCalledWith('/api/notebooks/import', {
      title: 'Imported',
      blocks: [{ type: 'paragraph', content: 'Hi' }],
    })
    expect(result).toEqual(mockData)
  })

  it('executeCommand sends POST with block_id', async () => {
    const mockData = { id: 'exec-1', status: 'success' }
    mockClient.post.mockResolvedValue({ data: { data: mockData } })
    const result = await executeCommand('blk-1')
    expect(mockClient.post).toHaveBeenCalledWith('/api/notebooks/execute', {
      block_id: 'blk-1',
    })
    expect(result).toEqual(mockData)
  })

  it('listExecutions fetches execution history', async () => {
    const mockData = [{ id: 'exec-1', block_id: 'blk-1' }]
    mockClient.get.mockResolvedValue({ data: { data: mockData } })
    const result = await listExecutions('blk-1')
    expect(mockClient.get).toHaveBeenCalledWith('/api/notebooks/executions/blk-1')
    expect(result).toEqual(mockData)
  })
})
