import client from './client'

export interface FileEntry {
  name: string
  path: string
  file_type: 'file' | 'directory'
  size: number | null
}

export interface FileListResponse {
  path: string
  entries: FileEntry[]
}

export interface RenameRequest {
  old_path: string
  new_path: string
}

export interface MkdirRequest {
  path: string
}

export interface TouchRequest {
  path: string
}

export function listFiles(resourceId: string, path: string): Promise<FileListResponse> {
  return client.get(`/resources/${resourceId}/files`, { params: { path } }).then(r => r.data.data)
}

export function mkdirFile(resourceId: string, path: string): Promise<void> {
  return client.post(`/resources/${resourceId}/files/mkdir`, { path }).then(() => {})
}

export function touchFile(resourceId: string, path: string): Promise<void> {
  return client.post(`/resources/${resourceId}/files/touch`, { path }).then(() => {})
}

export function deleteFile(resourceId: string, path: string): Promise<void> {
  return client.delete(`/resources/${resourceId}/files`, { params: { path } }).then(() => {})
}

export function renameFile(resourceId: string, oldPath: string, newPath: string): Promise<void> {
  return client.put(`/resources/${resourceId}/files/rename`, { old_path: oldPath, new_path: newPath }).then(() => {})
}

export function downloadFileUrl(resourceId: string, path: string): string {
  return `${client.defaults.baseURL}/resources/${resourceId}/files/download?path=${encodeURIComponent(path)}`
}

export async function downloadFile(resourceId: string, path: string): Promise<void> {
  const response = await client.get(`/resources/${resourceId}/files/download`, {
    params: { path },
    responseType: 'blob',
  })
  const filename = path.split('/').pop() || 'download'
  const blob = new Blob([response.data])
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

export function uploadFile(resourceId: string, dirPath: string, file: File): Promise<void> {
  const formData = new FormData()
  formData.append('file', file)
  return client.post(`/resources/${resourceId}/files/upload`, formData, {
    params: { path: dirPath },
    headers: { 'Content-Type': 'multipart/form-data' },
  }).then(() => {})
}
