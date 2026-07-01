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

// ── User Settings ──────────────────────────────────────────

export interface UserSettings {
  // Security
  session_timeout?: number
  audit_enabled?: boolean
  config_encryption?: boolean
  sidebar_collapsible?: boolean
  // Terminal
  terminal_font_size?: number
  terminal_font_family?: string
  terminal_cursor_blink?: boolean
  terminal_keepalive?: number
  // Appearance
  theme?: string
  lang?: string
}

export async function getUserSettings(): Promise<UserSettings> {
  const res = await client.get('/api/user/settings')
  return res.data.data
}

export async function updateUserSettings(settings: UserSettings): Promise<UserSettings> {
  const res = await client.put('/api/user/settings', settings)
  return res.data.data
}
