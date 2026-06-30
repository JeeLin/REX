import { ref, watch, type Ref } from 'vue'

/**
 * Creates a debounced ref that updates after a delay.
 * Useful for search inputs to avoid filtering on every keystroke.
 */
export function useDebouncedRef<T>(initialValue: T, delay = 300): Ref<T> {
  const source = ref<T>(initialValue) as Ref<T>
  const debounced = ref<T>(initialValue) as Ref<T>
  let timer: ReturnType<typeof setTimeout> | null = null

  watch(source, (val) => {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      debounced.value = val
      timer = null
    }, delay)
  })

  return debounced
}
