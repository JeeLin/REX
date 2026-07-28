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
}

export const settingsApi = {
  get: () => api.get<Settings>('/settings'),
  update: (data: Partial<Settings>) => {
    // Convert session_timeout from number to string for backend API (HashMap<String, String>)
    const apiData: Record<string, unknown> = { ...data }
    if ('session_timeout' in apiData && typeof apiData.session_timeout === 'number') {
      apiData.session_timeout = String(apiData.session_timeout)
    }
    return api.put<{ ok: boolean }>('/settings', apiData)
  },
}
