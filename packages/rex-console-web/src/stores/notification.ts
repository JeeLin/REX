import { defineStore } from 'pinia'
import { ref } from 'vue'

export type NotificationType = 'success' | 'error' | 'warning' | 'info'

export interface Notification {
  id: string
  type: NotificationType
  message: string
  duration?: number
}

export const useNotificationStore = defineStore('notification', () => {
  const notifications = ref<Notification[]>([])

  function push(type: NotificationType, message: string, duration = 5000) {
    const id = Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
    notifications.value.push({ id, type, message, duration })
    if (duration > 0) {
      setTimeout(() => remove(id), duration)
    }
  }

  function remove(id: string) {
    notifications.value = notifications.value.filter(n => n.id !== id)
  }

  function clear() {
    notifications.value = []
  }

  function success(message: string) { push('success', message) }
  function error(message: string) { push('error', message, 8000) }
  function warning(message: string) { push('warning', message) }
  function info(message: string) { push('info', message) }

  return { notifications, push, remove, clear, success, error, warning, info }
})
