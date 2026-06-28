import { useId as vueUseId } from 'vue'

let counter = 0

/**
 * Generate a unique ID for ARIA associations.
 * Uses Vue's built-in useId when available, falls back to a simple counter.
 */
export function useId(prefix = 'id'): string {
  try {
    // Vue 3.5+ useId() returns a string like ':r0:'
    return `${prefix}-${vueUseId()}`
  } catch {
    // Fallback for older Vue or non-setup context
    return `${prefix}-${++counter}`
  }
}
