import { ref, computed, onMounted, onUnmounted, watch } from 'vue'

interface VirtualListOptions {
  /** Total number of items */
  itemHeight: number | ((index: number) => number)
  /** Height of each item in px (or function returning height) */
  containerHeight: number
  /** Number of extra items to render above/below viewport */
  overscan?: number
}

/**
 * Virtual list composable for rendering large lists efficiently.
 * Only renders visible items plus a configurable overscan buffer.
 */
export function useVirtualList<T>(items: T[], options: VirtualListOptions) {
  const {
    itemHeight,
    containerHeight,
    overscan = 5,
  } = options

  const scrollTop = ref(0)

  const getItemHeight = (index: number): number => {
    return typeof itemHeight === 'function' ? itemHeight(index) : itemHeight
  }

  /** Total scrollable height */
  const totalHeight = computed(() => {
    let h = 0
    for (let i = 0; i < items.length; i++) {
      h += getItemHeight(i)
    }
    return h
  })

  /** Find the start index based on scroll position */
  const startIndex = computed(() => {
    let h = 0
    for (let i = 0; i < items.length; i++) {
      h += getItemHeight(i)
      if (h > scrollTop.value) return i
    }
    return 0
  })

  /** Rendered items with overscan buffer */
  const visibleItems = computed(() => {
    const start = Math.max(0, startIndex.value - overscan)
    let h = 0
    const topPadding = (() => {
      let p = 0
      for (let i = 0; i < start; i++) {
        p += getItemHeight(i)
      }
      return p
    })()

    // Calculate from start index
    let accumulated = 0
    for (let i = 0; i < start; i++) {
      accumulated += getItemHeight(i)
    }

    const result: Array<{ item: T; index: number; offsetY: number; height: number }> = []
    let y = accumulated

    for (let i = start; i < items.length; i++) {
      const h = getItemHeight(i)
      result.push({ item: items[i], index: i, offsetY: y, height: h })
      y += h
      // Stop when we've exceeded container + overscan
      if (y - accumulated > containerHeight + overscan * getItemHeight(i)) break
    }

    return result
  })

  function onScroll(e: Event) {
    const el = e.target as HTMLElement
    scrollTop.value = el.scrollTop
  }

  function scrollToTop() {
    scrollTop.value = 0
  }

  return {
    visibleItems,
    totalHeight,
    onScroll,
    scrollToTop,
  }
}
