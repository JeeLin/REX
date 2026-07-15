const API_BASE = '/api/redis'

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

export interface DbInfo {
  index: number
  keys: number
  expires: number
}

export interface KeyInfo {
  key: string
  type_name: string
}

export interface RedisInfo {
  redis_version: string
  os: string
  process_id: string
  connected_clients: string
  used_memory: string
  used_memory_peak: string
  total_commands_processed: string
  keyspace: { db: string; keys: number; expires: number }[]
}

export interface RedisValue {
  type: 'String' | 'List' | 'Set' | 'ZSet' | 'Hash'
  value: unknown
}

export async function connect(host: string, port: number, password?: string, db?: number): Promise<string> {
  const res = await fetch(`${API_BASE}/connect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ host, port, password, db }),
  })
  if (!res.ok) throw new Error((await res.json()).error?.message || 'Connection failed')
  return (await res.json()).session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await fetch(`${API_BASE}/disconnect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId }),
  })
}

export async function getDatabases(sessionId: string): Promise<DbInfo[]> {
  const res = await fetch(`${API_BASE}/databases?session_id=${sessionId}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to fetch databases')
  return await res.json()
}

export async function selectDb(sessionId: string, db: number): Promise<void> {
  await fetch(`${API_BASE}/select`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, db }),
  })
}

export async function scan(sessionId: string, pattern = '*', count = 100): Promise<KeyInfo[]> {
  const res = await fetch(`${API_BASE}/scan?session_id=${sessionId}&pattern=${encodeURIComponent(pattern)}&count=${count}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to scan keys')
  return await res.json()
}

export async function getValue(sessionId: string, key: string): Promise<RedisValue> {
  const res = await fetch(`${API_BASE}/key?session_id=${sessionId}&key=${encodeURIComponent(key)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to get value')
  return await res.json()
}

export async function setValue(sessionId: string, key: string, value: string): Promise<void> {
  await fetch(`${API_BASE}/set`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, key, value }),
  })
}

export async function delKeys(sessionId: string, keys: string[]): Promise<number> {
  const res = await fetch(`${API_BASE}/del`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, keys }),
  })
  if (!res.ok) throw new Error('Failed to delete keys')
  return (await res.json()).deleted
}

export async function getTtl(sessionId: string, key: string): Promise<number> {
  const res = await fetch(`${API_BASE}/ttl?session_id=${sessionId}&key=${encodeURIComponent(key)}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to get TTL')
  return (await res.json()).ttl
}

export async function setTtl(sessionId: string, key: string, seconds: number): Promise<void> {
  await fetch(`${API_BASE}/set-ttl`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, key, seconds }),
  })
}

export async function getInfo(sessionId: string): Promise<RedisInfo> {
  const res = await fetch(`${API_BASE}/info?session_id=${sessionId}`, { headers: authHeaders() })
  if (!res.ok) throw new Error('Failed to get info')
  return await res.json()
}

export async function runCommand(sessionId: string, args: string[]): Promise<string> {
  const res = await fetch(`${API_BASE}/command`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, args }),
  })
  if (!res.ok) throw new Error('Failed to run command')
  return (await res.json()).result
}
