import client from './client'

export interface LoginPayload {
  password: string
}

export interface LoginResult {
  token: string
  expires_at: string
}

export async function loginApi(payload: LoginPayload): Promise<LoginResult> {
  const { data } = await client.post<{ data: LoginResult }>('/auth/login', payload)
  return data.data
}
