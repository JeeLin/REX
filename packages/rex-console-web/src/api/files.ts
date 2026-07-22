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
  storage_class?: string | null
  acl?: string | null
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

export async function uploadFile(sessionId: string, remotePath: string, file: File, offset: number = 0): Promise<{ upload_id?: string }> {
  const form = new FormData()
  form.append('session_id', sessionId)
  form.append('path', remotePath)
  if (offset > 0) form.append('offset', offset.toString())
  form.append('file', file)
  const res = await fetch(`${API_BASE}/upload`, { method: 'POST', headers: authHeaders(), body: form })
  if (!res.ok) throw new Error('Upload failed')
  return await res.json()
}

/** Upload with progress tracking via XMLHttpRequest */
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
      } else reject(new Error('Upload failed'))
    }
    xhr.onerror = () => reject(new Error('Upload failed'))
    const form = new FormData()
    form.append('session_id', sessionId)
    form.append('path', remotePath)
    form.append('file', file)
    xhr.send(form)
  })
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

export async function chmod(sessionId: string, path: string, mode: string): Promise<void> {
  const res = await fetch(`${API_BASE}/chmod`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, mode }),
  })
  if (!res.ok) throw new Error('Chmod failed')
}

export async function presignedUrl(sessionId: string, path: string, expires?: number): Promise<string> {
  const res = await fetch(`${API_BASE}/presigned-url`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, expires_in: expires || 3600 }),
  })
  if (!res.ok) throw new Error('Failed to generate presigned URL')
  return (await res.json()).url
}

export async function listMultipartUploads(sessionId: string, prefix: string): Promise<Array<{ key: string; upload_id: string }>> {
  const res = await fetch(`${API_BASE}/s3/multipart-uploads?session_id=${sessionId}&prefix=${encodeURIComponent(prefix)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to list multipart uploads')
  return (await res.json()).uploads
}

export async function resumeMultipartUpload(
  sessionId: string,
  remotePath: string,
  uploadId: string,
  file: File,
  startPart: number = 1,
): Promise<void> {
  const form = new FormData()
  form.append('session_id', sessionId)
  form.append('path', remotePath)
  form.append('upload_id', uploadId)
  form.append('start_part', startPart.toString())
  form.append('file', file)
  const res = await fetch(`${API_BASE}/s3/resume-upload`, { method: 'POST', headers: authHeaders(), body: form })
  if (!res.ok) throw new Error('Failed to resume multipart upload')
}

export async function abortMultipartUpload(sessionId: string, path: string, uploadId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/s3/abort-upload`, {
    method: 'POST', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, upload_id: uploadId }),
  })
  if (!res.ok) throw new Error('Failed to abort multipart upload')
}

export async function getAcl(sessionId: string, path: string): Promise<string> {
  const res = await fetch(`${API_BASE}/acl?session_id=${sessionId}&path=${encodeURIComponent(path)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to get ACL')
  return (await res.json()).acl
}

export async function putAcl(sessionId: string, path: string, acl: string): Promise<void> {
  const res = await fetch(`${API_BASE}/acl`, {
    method: 'PUT', headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, path, acl }),
  })
  if (!res.ok) throw new Error('Failed to set ACL')
}
