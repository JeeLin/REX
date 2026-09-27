// This store only owns the shortcut panel's open/closed state.
// Key bindings are NOT defined here: each page listens for its own keys
// locally, and ShortcutPanel.vue only displays the resulting key table.
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useShortcutsStore = defineStore('shortcuts', () => {
  const show = ref(false)

  function toggle() {
    show.value = !show.value
  }

  function close() {
    show.value = false
  }

  return { show, toggle, close }
})
