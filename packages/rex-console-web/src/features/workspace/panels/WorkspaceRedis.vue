<template>
  <div class="ws-redis">
    <RedisConsole
      :resource-id="resourceId"
      :resource-name="resourceName"
    />
  </div>
</template>

<script setup lang="ts">
import RedisConsole from '@/features/redis/RedisConsole.vue'

defineProps<{
  resourceId: string
  resourceName: string
}>()
</script>

<style scoped>
.ws-redis {
  display: flex;
  flex-direction: column;
  height: 100%;
}
</style>
