<template>
  <div class="ws-docker">
    <DockerConsole
      :resource-id="resourceId"
      :resource-name="resourceName"
    />
  </div>
</template>

<script setup lang="ts">
import DockerConsole from '@/features/docker/DockerConsole.vue'

defineProps<{
  resourceId: string
  resourceName: string
}>()
</script>

<style scoped>
.ws-docker {
  display: flex;
  flex-direction: column;
  height: 100%;
}
</style>
