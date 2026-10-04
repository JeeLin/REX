//! 文件传输 API 调用封装。
//! 直连 fetch：presigned / S3 直传 / direct-download 不经过浏览器代理，仅错误处理归一为 ApiError。

import { ApiError } from './client'

const API_BASE = '/api/files'

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

async function raise(res: Response): Promise<never> {
  const body = await res.json().catch(() => ({})) as { error?: { code?: string; message?: string } } | null
  const code = (body && body.error && body.error.code) || `HTTP_${res.status}`
  const message = (body && body.error && body.error.message) || res.statusText || 'Request failed'
  throw new ApiError(code, message)
}

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: string | null
  permissions: string | null
  storage_class?: string | null
  acl?: string | null
}

export async function connect(resourceId: string): Promise<string> {
  const res = await fetch(`${API_BASE}/connect`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ resource_id: resourceId }),
  })
  if (!res.ok) throw await raise(res)
  return (await res.json()).session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await fetch(`${API_BASE}/disconnect`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId }),
  })
}

// --- Connector capability (v0.91.0 T3/T6): capability-based UI instead of isS3 ---

export type FileCapability = 'chmod' | 'presigned_url' | 'acl' | 'multipart'

export interface FileCapabilities {
  chmod: boolean
  presigned_url: boolean
  acl: boolean
  multipart: boolean
}

export interface ConnectorCapabilityResponse {
  protocol: string
  capabilities: FileCapabilities
}

/** Query a connector's static capabilities (GET /api/files/connector/{resource_id}/capability). */
export async function getCapability(resourceId: string): Promise<ConnectorCapabilityResponse> {
  const res = await fetch(`${API_BASE}/connector/${encodeURIComponent(resourceId)}/capability`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

export async function listFiles(sessionId: string, path: string): Promise<FileEntry[]> {
  const res = await fetch(`${API_BASE}/list?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

export async function statFile(sessionId: string, path: string): Promise<FileEntry> {
  const res = await fetch(`${API_BASE}/stat?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

export async function uploadFile(sessionId: string, remotePath: string, file: File, offset: number = 0): Promise<{ upload_id?: string }> {
  const form = new FormData()
  form.append('session_id', sessionId)
  form.append('path', remotePath)
  if (offset > 0) form.append('offset', offset.toString())
  form.append('file', file)
  const res = await fetch(`${API_BASE}/upload`, { method: 'POST', headers: authHeaders(), body: form })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

/** Upload with progress tracking via XMLHttpRequest (presigned/S3 直传不走 ApiClient) */
export function uploadFileWithProgress(
  sessionId: string,
  remotePath: string,
  file: File,
  onProgress?: (percent: number, transferred: number) => void,
): Promise<{ upload_id?: string }> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', `${API_BASE}/upload`)
    const headers = authHeaders()
    for (const [key, value] of Object.entries(headers)) {
      xhr.setRequestHeader(key, value)
    }
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable && onProgress) {
        onProgress(Math.round((e.loaded / e.total) * 100), e.loaded)
      }
    }
    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          const result = JSON.parse(xhr.responseText)
          resolve(result)
        } catch {
          resolve({})
        }
      } else reject(new ApiError('UPLOAD_FAILED', 'Upload failed'))
    }
    xhr.onerror = () => reject(new ApiError('UPLOAD_FAILED', 'Upload failed'))
    const form = new FormData()
    form.append('session_id', sessionId)
    form.append('path', remotePath)
    form.append('file', file)
    xhr.send(form)
  })
}

export async function downloadFile(sessionId: string, path: string, offset?: number): Promise<Blob> {
  const headers: Record<string, string> = authHeaders()
  if (offset && offset > 0) {
    headers['Range'] = `bytes=${offset}-`
  }
  const res = await fetch(`${API_BASE}/download?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers })
  if (!res.ok && res.status !== 206) throw await raise(res)
  return await res.blob()
}

export async function deleteFile(sessionId: string, path: string): Promise<void> {
  const res = await fetch(`${API_BASE}/delete`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path }),
  })
  if (!res.ok) throw await raise(res)
}

export async function renameFile(sessionId: string, from: string, to: string): Promise<void> {
  const res = await fetch(`${API_BASE}/rename`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, from, to }),
  })
  if (!res.ok) throw await raise(res)
}

export async function mkdir(sessionId: string, path: string): Promise<void> {
  const res = await fetch(`${API_BASE}/mkdir`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path }),
  })
  if (!res.ok) throw await raise(res)
}

export async function chmod(sessionId: string, path: string, mode: string): Promise<void> {
  const res = await fetch(`${API_BASE}/chmod`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, mode }),
  })
  if (!res.ok) throw await raise(res)
}

export async function presignedUrl(sessionId: string, path: string, expires?: number): Promise<string> {
  const res = await fetch(`${API_BASE}/presigned-url`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, expires_in: expires || 3600 }),
  })
  if (!res.ok) throw await raise(res)
  return (await res.json()).url
}

export async function listMultipartUploads(sessionId: string, prefix: string): Promise<Array<{ key: string; upload_id: string }>> {
  const res = await fetch(`${API_BASE}/s3/multipart-uploads?session_id=${sessionId}&prefix=${encodeURIComponent(prefix)}`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return (await res.json()).uploads
}

export async function resumeMultipartUpload(
  sessionId: string,
  remotePath: string,
  uploadId: string,
  file: File,
): Promise<void> {
  const form = new FormData()
  form.append('session_id', sessionId)
  form.append('path', remotePath)
  form.append('upload_id', uploadId)
  form.append('file', file)
  const res = await fetch(`${API_BASE}/s3/resume-upload`, { method: 'POST', headers: authHeaders(), body: form })
  if (!res.ok) throw await raise(res)
}

export async function abortMultipartUpload(sessionId: string, path: string, uploadId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/s3/abort-upload`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, upload_id: uploadId }),
  })
  if (!res.ok) throw await raise(res)
}

export async function getAcl(sessionId: string, path: string): Promise<string> {
  const res = await fetch(`${API_BASE}/acl?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return (await res.json()).acl
}

export async function putAcl(sessionId: string, path: string, acl: string): Promise<void> {
  const res = await fetch(`${API_BASE}/acl`, {
    method: 'PUT', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, acl }),
  })
  if (!res.ok) throw await raise(res)
}

export async function readForEdit(sessionId: string, path: string): Promise<{
  content: string; filename: string; size: number
}> {
  const res = await fetch(`${API_BASE}/read-for-edit?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

export async function saveFromEdit(sessionId: string, path: string, content: string): Promise<void> {
  const res = await fetch(`${API_BASE}/save-from-edit`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, content }),
  })
  if (!res.ok) throw await raise(res)
}

// --- Transfer action (v0.91.0 T2): server-side move/copy ---
// File bytes never transit the browser: the task is created server-side and
// progress is polled from GET /api/files/transfer/{id}.

export type TransferOp = 'move' | 'copy'
export type TransferConflict = 'overwrite' | 'skip' | 'rename' | 'fail'
export type TransferTaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'canceled'

export interface TransferEndpoint {
  resource_id: string
  path: string
}

export interface TransferActionBody {
  op: TransferOp
  src: TransferEndpoint
  dst: TransferEndpoint
  conflict: TransferConflict
}

export interface TransferActionCreated {
  id: string
  status: string
}

/** Create a server-side move/copy task (POST /api/files/transfer/action). */
export async function transferAction(body: TransferActionBody): Promise<TransferActionCreated> {
  const res = await fetch(`${API_BASE}/transfer/action`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify(body),
  })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

/**
 * Transfer task record returned by GET /api/files/transfer/{id} (T1 `TransferTaskRecord`).
 * NOTE: progress fields are FLAT top-level (`total_bytes`, `transferred_bytes`,
 * `speed_bytes_per_sec`, `eta_seconds`), not nested under a `progress` object.
 */
export interface TransferTaskRecord {
  id: string
  source_resource_id: string
  target_resource_id: string
  source_path: string
  target_path: string
  conflict_policy: string
  status: TransferTaskStatus
  total_bytes: number
  transferred_bytes: number
  speed_bytes_per_sec: number
  eta_seconds: number | null
  error: string | null
  created_at: string
  updated_at: string
}

/** Poll a transfer task (GET /api/files/transfer/{id}). */
export async function getTransferTask(id: string): Promise<TransferTaskRecord> {
  const res = await fetch(`${API_BASE}/transfer/${encodeURIComponent(id)}`, { headers: authHeaders() })
  if (!res.ok) throw await raise(res)
  return await res.json()
}

/** Cancel a transfer task (POST /api/files/transfer/{id}/cancel). */
export async function cancelTransferTask(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/transfer/${encodeURIComponent(id)}/cancel`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
  })
  if (!res.ok) throw await raise(res)
}
