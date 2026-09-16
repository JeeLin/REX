const API_BASE = '/api/mongodb'

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('rex-token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

export async function connect(resourceId: string): Promise<string> {
  const res = await fetch(`${API_BASE}/connect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ resource_id: resourceId }),
  })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body?.error?.message || `HTTP ${res.status}`)
  }
  const data = await res.json()
  return data.session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await fetch(`${API_BASE}/disconnect`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId }),
  })
}

export async function getDatabases(sessionId: string): Promise<string[]> {
  const res = await fetch(`${API_BASE}/databases?session_id=${sessionId}`, { headers: authHeaders() })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  const data = await res.json()
  return Array.isArray(data) ? data : data.databases || []
}

export async function getCollections(sessionId: string, database: string): Promise<string[]> {
  const res = await fetch(`${API_BASE}/collections?session_id=${sessionId}&database=${database}`, { headers: authHeaders() })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  const data = await res.json()
  return Array.isArray(data) ? data : data.collections || []
}

export interface QueryResult {
  documents?: Record<string, unknown>[]
  count?: number
  error?: string
}

export async function query(sessionId: string, database: string, collection: string, operation: string, filter?: Record<string, unknown>, options?: { sort?: Record<string, unknown>; projection?: Record<string, unknown>; limit?: number }): Promise<QueryResult> {
  const res = await fetch(`${API_BASE}/query`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders() },
    body: JSON.stringify({ session_id: sessionId, database, collection, operation, filter, ...options }),
  })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return res.json()
}
