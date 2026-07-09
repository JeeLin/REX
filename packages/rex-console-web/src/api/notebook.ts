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
  title: string
  description: string | null
  blocks: BlockExport[]
}

export interface BlockExport {
  type: string
  content: string | null
  protocol: string | null
  resource_name: string | null
}

// ── API 函数 ──────────────────────────────────────────────

export async function listNotebooks(): Promise<Notebook[]> {
  const res = await client.get<{ data: Notebook[] }>('/api/notebooks')
  return res.data.data
}

export async function getNotebook(id: string): Promise<NotebookWithBlocks> {
  const res = await client.get<{ data: NotebookWithBlocks }>(`/api/notebooks/${id}`)
  return res.data.data
}

export async function createNotebook(data: {
  title?: string
  description?: string
}): Promise<Notebook> {
  const res = await client.post<{ data: Notebook }>('/api/notebooks', data)
  return res.data.data
}

export async function updateNotebook(
  id: string,
  data: { title?: string; description?: string }
): Promise<Notebook> {
  const res = await client.put<{ data: Notebook }>(`/api/notebooks/${id}`, data)
  return res.data.data
}

export async function deleteNotebook(id: string): Promise<void> {
  await client.delete(`/api/notebooks/${id}`)
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
    `/api/notebooks/${notebookId}/blocks`,
    { blocks }
  )
  return res.data.data
}

export async function exportNotebook(id: string): Promise<NotebookExport> {
  const res = await client.get<{ data: NotebookExport }>(`/api/notebooks/export/${id}`)
  return res.data.data
}

export async function importNotebook(data: {
  title?: string
  description?: string
  blocks?: BlockExport[]
}): Promise<Notebook> {
  const res = await client.post<{ data: Notebook }>('/api/notebooks/import', data)
  return res.data.data
}

export async function executeCommand(blockId: string): Promise<NotebookExecution> {
  const res = await client.post<{ data: NotebookExecution }>('/api/notebooks/execute', {
    block_id: blockId,
  })
  return res.data.data
}

export async function listExecutions(blockId: string): Promise<NotebookExecution[]> {
  const res = await client.get<{ data: NotebookExecution[] }>(
    `/api/notebooks/executions/${blockId}`
  )
  return res.data.data
}
