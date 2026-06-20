import { ref } from 'vue'

export interface MenuItem {
  label?: string
  icon?: string
  action?: () => void
  disabled?: boolean
  danger?: boolean
  separator?: boolean
  children?: MenuItem[]
}

const visible = ref(false)
const x = ref(0)
const y = ref(0)
const items = ref<MenuItem[]>([])

function show(event: MouseEvent, menuItems: MenuItem[]) {
  event.preventDefault()
  event.stopPropagation()

  // Calculate position, avoid going out of viewport
  const offsetX = 4
  const offsetY = 4
  const menuWidth = 220
  const menuHeight = menuItems.length * 36

  let posX = event.clientX + offsetX
  let posY = event.clientY + offsetY

  if (posX + menuWidth > window.innerWidth) {
    posX = event.clientX - menuWidth - offsetX
  }
  if (posY + menuHeight > window.innerHeight) {
    posY = event.clientY - menuHeight - offsetY
  }

  x.value = Math.max(0, posX)
  y.value = Math.max(0, posY)
  items.value = menuItems
  visible.value = true
}

function hide() {
  visible.value = false
}

export function useContextMenu() {
  return { visible, x, y, items, show, hide }
}
