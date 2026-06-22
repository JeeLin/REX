import client from './client'

export interface UserProfile {
  username: string
}

export interface ChangePasswordRequest {
  current_password: string
  new_password: string
}

export async function getUserProfile(): Promise<UserProfile> {
  const res = await client.get('/api/user/profile')
  return res.data.data
}

export async function updateUserProfile(username: string): Promise<UserProfile> {
  const res = await client.put('/api/user/profile', { username })
  return res.data.data
}

export async function changePassword(data: ChangePasswordRequest): Promise<void> {
  await client.put('/api/user/password', data)
}
