import { ref, onBeforeUnmount } from 'vue'

export function useSftpDrawer() {
  const show = ref(false)
  const height = ref(240)
  let dragStartY = 0
  let dragStartH = 0

  function toggle() {
    show.value = !show.value
  }

  function startDrag(e: MouseEvent) {
    dragStartY = e.clientY
    dragStartH = height.value
    document.addEventListener('mousemove', onDrag)
    document.addEventListener('mouseup', onDragEnd)
    document.body.style.cursor = 'row-resize'
    document.body.style.userSelect = 'none'
  }

  function onDrag(e: MouseEvent) {
    const delta = dragStartY - e.clientY
    height.value = Math.min(700, Math.max(120, dragStartH + delta))
  }

  function onDragEnd() {
    document.removeEventListener('mousemove', onDrag)
    document.removeEventListener('mouseup', onDragEnd)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }

  onBeforeUnmount(() => {
    document.removeEventListener('mousemove', onDrag)
    document.removeEventListener('mouseup', onDragEnd)
  })

  return { show, height, toggle, startDrag }
}
