import client from './client'

export interface TlsStatus {
  mode: string
  domain?: string
  cert_ready: boolean
  cert_expires_at?: string
  cert_issuer?: string
  port_80_required: boolean
}

export async function getTlsStatus(): Promise<TlsStatus> {
  const res = await client.get('/settings/tls')
  return res.data.data
}
