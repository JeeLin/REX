import client from './client'

export interface TransferEndpoint {
  connector_type: 'local' | 'sftp'
  resource_id?: string
  sftp_host?: string
  sftp_port?: number
  sftp_username?: string
  path: string
}

export interface TransferProgress {
  total_bytes: number
  transferred_bytes: number
}

export interface TransferTask {
  id: string
  source: TransferEndpoint
  target: TransferEndpoint
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'
  status_detail?: string
  progress: TransferProgress
  created_at: string
  updated_at: string
}

export function listTransfers(): Promise<TransferTask[]> {
  return client.get('/transfers').then(r => r.data.data)
}

export function createTransfer(source: TransferEndpoint, target: TransferEndpoint): Promise<TransferTask> {
  return client.post('/transfers', { source, target }).then(r => r.data.data)
}

export function getTransfer(id: string): Promise<TransferTask> {
  return client.get(`/transfers/${id}`).then(r => r.data.data)
}

export function cancelTransfer(id: string): Promise<void> {
  return client.delete(`/transfers/${id}`).then(() => {})
}

export function removeTransfer(id: string): Promise<void> {
  return client.delete(`/transfers/${id}/remove`).then(() => {})
}


export interface TransferStats {
  max_concurrent: number
  available_permits: number
  active_transfers: number
  pending_transfers: number
}

export function getTransferStats(): Promise<TransferStats> {
  return client.get('/transfers/stats').then(r => r.data.data)
}

export interface TransferConcurrency {
  max_concurrent: number
}

export function getTransferConcurrency(): Promise<number> {
  return client.get('/transfers/concurrency').then(r => r.data.data.max_concurrent)
}

export function setTransferConcurrency(maxConcurrent: number): Promise<number> {
  return client.put('/transfers/concurrency', { max_concurrent: maxConcurrent }).then(r => r.data.data.max_concurrent)
}