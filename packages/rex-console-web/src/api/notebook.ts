import client from './client'

// ── 类型定义 ──────────────────────────────────────────────

export interface Notebook {
  id: string
  title: string
  description: string | null
  tags: string
  created_at: string
  updated_at: string
}

export interface NotebookBlock {
  id: string
  notebook_id: string
  block_type: string
  content: string
  resource_id: string | null
  protocol: string | null
  order_index: number
  created_at: string
  updated_at: string
}

export interface NotebookWithBlocks extends Notebook {
  blocks: NotebookBlock[]
}

/** Editor-level block data model. */
export interface EditorBlock {
  id: string
  type: 'heading' | 'paragraph' | 'code' | 'command'
  content: string
  level?: number
  resourceId?: string
  protocol?: string
  orderIndex: number
}

export interface NotebookExecution {
  id: string
  block_id: string
  status: string
  output: string
  duration_ms: number | null
  executed_at: string
}

export interface NotebookExport {
  'rex-notebook': string
  [key: string]: unknown
}

export interface BlockExport {
  type: string
  content: string
  level?: number
  resourceId?: string
  protocol?: string
}

// ── API 函数 ──────────────────────────────────────────────

export async function listNotebooks(): Promise<Notebook[]> {
  const res = await client.get<{ data: Notebook[] }>('/notebooks')
  return res.data.data
}

export async function getNotebook(id: string): Promise<NotebookWithBlocks> {
  const res = await client.get<{ data: NotebookWithBlocks }>(`/notebooks/${id}`)
  return res.data.data
}

export async function createNotebook(data: {
  title: string
  description?: string
}): Promise<Notebook> {
  const res = await client.post<{ data: Notebook }>('/notebooks', data)
  return res.data.data
}

export async function updateNotebook(
  id: string,
  data: { title?: string; description?: string }
): Promise<Notebook> {
  const res = await client.put<{ data: Notebook }>(`/notebooks/${id}`, data)
  return res.data.data
}

export async function deleteNotebook(id: string): Promise<void> {
  await client.delete(`/notebooks/${id}`)
}

export async function updateBlocks(
  notebookId: string,
  blocks: Array<{
    id?: string
    block_type: string
    content?: string
    resource_id?: string
    protocol?: string
    order_index: number
  }>
): Promise<NotebookBlock[]> {
  const res = await client.put<{ data: NotebookBlock[] }>(
    `/notebooks/${notebookId}/blocks`,
    { blocks }
  )
  return res.data.data
}

export async function exportNotebook(id: string): Promise<NotebookExport> {
  const res = await client.get<{ data: NotebookExport }>(`/notebooks/export/${id}`)
  return res.data.data
}

export async function importNotebook(data: {
  title?: string
  description?: string
  blocks?: BlockExport[]
}): Promise<Notebook> {
  const res = await client.post<{ data: Notebook }>('/notebooks/import', data)
  return res.data.data
}

export async function executeCommand(blockId: string): Promise<NotebookExecution> {
  const res = await client.post<{ data: NotebookExecution }>('/notebooks/execute', {
    block_id: blockId,
  })
  return res.data.data
}

export async function listExecutions(blockId: string): Promise<NotebookExecution[]> {
  const res = await client.get<{ data: NotebookExecution[] }>(
    `/notebooks/executions/${blockId}`
  )
  return res.data.data
}
