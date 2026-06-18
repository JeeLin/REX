import client from './client'

export interface CreateSessionRequest {
  resource_id: string
  cols: number
  rows: number
}

export interface CreateSessionResult {
  session_id: string
}

export async function createSession(payload: CreateSessionRequest): Promise<CreateSessionResult> {
  const { data } = await client.post<{ data: CreateSessionResult }>('/ssh/sessions', payload)
  return data.data
}

export async function deleteSession(sessionId: string): Promise<void> {
  await client.delete(`/ssh/sessions/${sessionId}`)
}
