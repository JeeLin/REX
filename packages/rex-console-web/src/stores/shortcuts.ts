// This store owns the open/closed state of the AppLayout overlay panels: the
// shortcut panel (`show`) and the command palette (`paletteVisible`), so pages
// can tell whether an overlay swallows their key chords without DOM probing.
// Key bindings are NOT defined here: each page listens for its own keys
// locally, and ShortcutPanel.vue only displays the resulting key table.
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useShortcutsStore = defineStore('shortcuts', () => {
  const show = ref(false)
  const paletteVisible = ref(false)

  function toggle() {
    show.value = !show.value
  }

  function close() {
    show.value = false
  }

  function togglePalette() {
    paletteVisible.value = !paletteVisible.value
  }

  function closePalette() {
    paletteVisible.value = false
  }

  return { show, toggle, close, paletteVisible, togglePalette, closePalette }
})
