import { api } from './client'

export interface Settings {
  theme: string
  language: string
  terminal_font: string
  terminal_font_size: string
  terminal_theme: string
  terminal_opacity: number
  terminal_bg_image: string
}

export const settingsApi = {
  get: () => api.get<Settings>('/settings'),
  update: (data: Partial<Settings>) => api.put<{ ok: boolean }>('/settings', data),
}
