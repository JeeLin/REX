import type { NotebookExport, BlockExport } from '@/api/notebook'
import { exportNotebook as apiExportNotebook } from '@/api/notebook'

const VALID_BLOCK_TYPES: Record<string, true> = {
  heading: true,
  paragraph: true,
  code: true,
  command: true,
}

/**
 * 从后端获取笔记本导出数据并触发 JSON 文件下载。
 */
export async function exportNotebookToFile(notebookId: string, title?: string): Promise<void> {
  const data: NotebookExport = await apiExportNotebook(notebookId)
  const json = JSON.stringify(data, null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  const name = String(title || (data as Record<string, unknown>).title || 'notebook')
  a.download = `${name.replace(/[<>:"/\\|?*]/g, '_').trim() || 'notebook'}.rex-notebook.json`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

/** 后端导出文件格式的块结构 */
export interface NotebookFileBlock {
  type: string
  content?: string | null
  level?: number | null
  protocol?: string | null
  resource_name?: string | null
}

/** 后端导出文件格式 */
export interface NotebookFileData {
  'rex-notebook': string
  title?: string
  description?: string | null
  blocks?: NotebookFileBlock[]
}

/**
 * 从本地 JSON 文件导入笔记本数据。
 *
 * 校验流程：
 * 1. 文件内容可解析为 JSON
 * 2. 包含 `rex-notebook` 字段且值为 "1.0"
 * 3. `blocks` 存在且为数组
 * 4. 每个块必须包含 `type` 和 `content`
 *
 * @throws 导出的格式验证错误信息
 */
export async function importNotebookFromFile(file: File): Promise<{
  title: string
  description: string
  blocks: BlockExport[]
}> {
  if (!file.name.endsWith('.json')) {
    throw new Error('文件格式无效：请选择 .json 文件')
  }

  const text = await file.text()
  let data: NotebookFileData
  try {
    data = JSON.parse(text) as NotebookFileData
  } catch {
    throw new Error('文件内容不是有效的 JSON')
  }

  if (data['rex-notebook'] !== '1.0') {
    throw new Error(`不支持的导出版本：${String(data['rex-notebook'] ?? '未知')}（仅支持 1.0）`)
  }

  if (!Array.isArray(data.blocks) || data.blocks.length === 0) {
    throw new Error('笔记本不包含任何块')
  }

  for (const [i, block] of data.blocks.entries()) {
    if (!block.type || !(block.type in VALID_BLOCK_TYPES)) {
      throw new Error(`第 ${i + 1} 个块的类型无效：${String(block.type)}`)
    }
    if (block.content == null) {
      throw new Error(`第 ${i + 1} 个块缺少内容`)
    }
  }

  const blocks: BlockExport[] = data.blocks.map((b) => ({
    type: b.type,
    content: String(b.content ?? ''),
    protocol: b.protocol ?? undefined,
    resourceId: b.resource_name ?? undefined,
  }))

  return {
    title: data.title || file.name.replace(/\.rex-notebook\.json$/, '').replace(/\.json$/, ''),
    description: data.description || '',
    blocks,
  }
}
