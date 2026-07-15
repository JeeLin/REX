const API_BASE = '/api/files'

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
  modified: string | null
  permissions: string | null
}

export async function connect(req: {
  protocol: string; host: string; port: number; username?: string;
  password?: string; private_key?: string; bucket?: string; region?: string;
  endpoint?: string; access_key?: string; secret_key?: string;
}): Promise<string> {
  const res = await fetch(`${API_BASE}/connect`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify(req),
  })
  if (!res.ok) throw new Error((await res.json()).error?.message || 'Connection failed')
  return (await res.json()).session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await fetch(`${API_BASE}/disconnect`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId }),
  })
}

export async function listFiles(sessionId: string, path: string): Promise<FileEntry[]> {
  const res = await fetch(`${API_BASE}/list?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to list files')
  return await res.json()
}

export async function statFile(sessionId: string, path: string): Promise<FileEntry> {
  const res = await fetch(`${API_BASE}/stat?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to stat file')
  return await res.json()
}

export async function uploadFile(sessionId: string, remotePath: string, file: File): Promise<void> {
  const form = new FormData()
  form.append('session_id', sessionId)
  form.append('path', remotePath)
  form.append('file', file)
  const res = await fetch(`${API_BASE}/upload`, { method: 'POST', headers: authHeaders(), body: form })
  if (!res.ok) throw new Error('Upload failed')
}

export async function downloadFile(sessionId: string, path: string): Promise<Blob> {
  const res = await fetch(`${API_BASE}/download?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Download failed')
  return await res.blob()
}

export async function deleteFile(sessionId: string, path: string): Promise<void> {
  const res = await fetch(`${API_BASE}/delete`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path }),
  })
  if (!res.ok) throw new Error('Delete failed')
}

export async function renameFile(sessionId: string, from: string, to: string): Promise<void> {
  const res = await fetch(`${API_BASE}/rename`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, from, to }),
  })
  if (!res.ok) throw new Error('Rename failed')
}

export async function mkdir(sessionId: string, path: string): Promise<void> {
  const res = await fetch(`${API_BASE}/mkdir`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path }),
  })
  if (!res.ok) throw new Error('Mkdir failed')
}
