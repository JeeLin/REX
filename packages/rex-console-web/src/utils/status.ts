import type { StatusDotStatus } from '@/components/ui/StatusDot.vue'

export function agentStatus(status: string | null): StatusDotStatus {
  if (status === 'online') return 'online'
  if (status === 'connecting') return 'connecting'
  return 'offline'
}
