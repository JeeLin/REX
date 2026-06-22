<template>
  <router-view />
  <ContextMenu />
</template>

<script setup lang="ts">
import ContextMenu from '@/components/ContextMenu.vue'
import { useSessionTimeout } from '@/composables/useSessionTimeout'

useSessionTimeout()
</script>
