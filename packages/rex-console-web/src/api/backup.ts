
// ── 类型定义 ──────────────────────────────────────────────

export interface ImportCounts {
  created: number
  skipped: number
  updated: number
}

export interface ImportResult {
  environments: ImportCounts
  resources: ImportCounts
  settings: ImportCounts
  warnings: string[]
}

export interface PreviewItem {
  id: string
  name: string
  exists: boolean
  extra: string | null
}

export interface PreviewResult {
  hub_version: string
  created_at: string
  encrypted: boolean
  environments: PreviewItem[]
  resources: PreviewItem[]
  settings_count: number
}

// ── API 函数 ──────────────────────────────────────────────

/** 导出备份文件，返回 Blob 供下载 */
export async function exportBackup(options?: {
  envIds?: string[]
  password?: string
}): Promise<Blob> {
  const params = new URLSearchParams()
  if (options?.envIds?.length) {
    params.set('env_ids', options.envIds.join(','))
  }

  const body: Record<string, unknown> = {}
  if (options?.password) {
    body.password = options.password
  }

  const url = `/backup/export${params.toString() ? '?' + params.toString() : ''}`
  const token = localStorage.getItem('rex-token')
  const resp = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  })

  if (!resp.ok) {
    const err = await resp.json().catch(() => ({ error: { message: resp.statusText } }))
    throw new Error(err.error?.message || 'Export failed')
  }

  return resp.blob()
}

/** 预览备份文件内容 */
export async function previewBackup(
  file: File,
  password?: string,
): Promise<PreviewResult> {
  const form = new FormData()
  form.append('file', file)
  if (password) form.append('password', password)

  const token = localStorage.getItem('rex-token')
  const resp = await fetch('/backup/preview', {
    method: 'POST',
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body: form,
  })

  const json = await resp.json()
  if (!resp.ok) {
    throw new Error(json.error?.message || 'Preview failed')
  }

  return json.data
}

/** 导入备份文件 */
export async function importBackup(
  file: File,
  options?: {
    password?: string
    strategy?: 'skip_existing' | 'overwrite'
  },
): Promise<ImportResult> {
  const form = new FormData()
  form.append('file', file)
  if (options?.password) form.append('password', options.password)
  if (options?.strategy) form.append('strategy', options.strategy)

  const token = localStorage.getItem('rex-token')
  const resp = await fetch('/backup/import', {
    method: 'POST',
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body: form,
  })

  const json = await resp.json()
  if (!resp.ok) {
    throw new Error(json.error?.message || 'Import failed')
  }

  return json.data
}
