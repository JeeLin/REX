import { ref, onMounted, onBeforeUnmount } from 'vue'

/**
 * 监听虚拟键盘弹出/收起状态。
 * 使用 visualViewport API（iOS Safari / Android Chrome 均支持）。
 */
export function useVirtualKeyboard() {
  const isKeyboardVisible = ref(false)

  function onViewportResize() {
    const vh = window.visualViewport?.height ?? window.innerHeight
    // 键盘弹出时，viewport 高度会明显小于窗口高度
    isKeyboardVisible.value = vh < window.innerHeight * 0.75
  }

  onMounted(() => {
    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', onViewportResize)
    } else {
      window.addEventListener('resize', onViewportResize)
    }
  })

  onBeforeUnmount(() => {
    if (window.visualViewport) {
      window.visualViewport.removeEventListener('resize', onViewportResize)
    } else {
      window.removeEventListener('resize', onViewportResize)
    }
  })

  return { isKeyboardVisible }
}
