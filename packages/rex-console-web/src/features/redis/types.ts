import type { RedisValue } from '@/api/redis'

export interface KeyWithType {
  key: string
  type: string
}

export interface OutputEntry {
  id: number
  command: string
  response?: RedisValue
  error?: string
  elapsed_ms?: number
}
