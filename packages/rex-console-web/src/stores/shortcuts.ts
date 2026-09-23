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
