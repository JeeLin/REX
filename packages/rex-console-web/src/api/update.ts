import client from './client'

// ── 类型定义 ──────────────────────────────────────────────

export interface UpdateStatusResponse {
  current_version: string
  git_commit: string
  latest_version: string | null
  update_available: boolean
  last_checked: string | null
  auto_check_enabled: boolean
}

export interface AgentVersionInfo {
  agent_id: string
  name: string
  environment_id: string
  version: string
  sha256: string
  needs_update: boolean
  status: string
  last_seen_at: string | null
}

export interface DownloadProgress {
  status: string // "downloading" | "verifying" | "ready" | "error"
  percent: number
  message: string
}

// ── API 函数 ──────────────────────────────────────────────

/** 获取 Hub 更新状态 */
export function getUpdateStatus(): Promise<UpdateStatusResponse> {
  return client.get('/update/status').then((r) => r.data.data)
}

/** 手动触发更新检查 */
export function checkUpdate(): Promise<UpdateStatusResponse> {
  return client.get('/update/check').then((r) => r.data.data)
}

/** 获取所有 Agent 版本信息 */
export function listAgentVersions(): Promise<AgentVersionInfo[]> {
  return client.get('/update/agents').then((r) => r.data.data)
}

/** 下载新版本 */
export function downloadUpdate(version: string): Promise<DownloadProgress> {
  return client.post('/update/download', { version }).then((r) => r.data.data)
}

/** 应用更新（触发 supervisor 重启） */
export function applyUpdate(): Promise<DownloadProgress> {
  return client.post('/update/apply').then((r) => r.data.data)
}
