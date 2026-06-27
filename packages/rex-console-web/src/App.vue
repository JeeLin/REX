<template>
  <router-view />
  <ContextMenu />
  <ToastProvider />
</template>

<script setup lang="ts">
import ContextMenu from '@/components/ContextMenu.vue'
import ToastProvider from '@/components/ToastProvider.vue'
import { useSessionTimeout } from '@/composables/useSessionTimeout'

useSessionTimeout()
</script>
