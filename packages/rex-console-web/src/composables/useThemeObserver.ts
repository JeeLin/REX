import { onBeforeUnmount } from 'vue'

/**
 * 监听 data-theme 属性变化的 composable
 * 必须在 <script setup> 顶层调用（不能在 onMounted 中调用）
 */
export function useThemeObserver(callback: () => void): void {
  const observer = new MutationObserver(callback)
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
  onBeforeUnmount(() => observer.disconnect())
}
