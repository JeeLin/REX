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
