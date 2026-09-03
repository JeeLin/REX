import { api } from './client'

export interface Settings {
  theme: string
  language: string
  terminal_font: string
  terminal_font_size: string
  terminal_theme: string
  terminal_opacity: number
  terminal_bg_image: string
  session_timeout: number
  auto_update: boolean
  audit_logging: boolean
}

export interface UpdateInfo {
  has_update: boolean
  current_version: string
  latest_version: string
  download_url: string
}

export interface UpdateStatus {
  phase: string
  target_version: string
  old_version: string
  attempt: number
}

export const settingsApi = {
  get: () => api.get<Settings>('/settings'),
  update: (data: Partial<Settings>) => {
    // Convert non-string values for backend API (HashMap<String, String>)
    const apiData: Record<string, unknown> = { ...data }
    if ('session_timeout' in apiData && typeof apiData.session_timeout === 'number') {
      apiData.session_timeout = String(apiData.session_timeout)
    }
    if ('auto_update' in apiData && typeof apiData.auto_update === 'boolean') {
      apiData.auto_update = String(apiData.auto_update)
    }
    if ('audit_logging' in apiData && typeof apiData.audit_logging === 'boolean') {
      apiData.audit_logging = String(apiData.audit_logging)
    }
    if ('terminal_font_size' in apiData && typeof apiData.terminal_font_size === 'number') {
      apiData.terminal_font_size = String(apiData.terminal_font_size)
    }
    return api.put<{ ok: boolean }>('/settings', apiData)
  },
  changePassword: (currentPassword: string, newPassword: string) =>
    api.post<{ ok: boolean }>('/auth/change-password', {
      current_password: currentPassword,
      new_password: newPassword,
    }),
}

export const updateApi = {
  check: () => api.get<UpdateInfo>('/update/check'),
  trigger: () => api.post<{ ok: boolean }>('/update/trigger'),
  status: () => api.get<UpdateStatus>('/update/status'),
  rollback: () => api.post<{ ok: boolean }>('/update/rollback'),
}
