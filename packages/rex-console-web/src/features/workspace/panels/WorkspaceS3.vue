<template>
  <div class="ws-s3">
    <S3Console
      :resource-id="resourceId"
      :resource-name="resourceName"
    />
  </div>
</template>

<script setup lang="ts">
import S3Console from '@/features/s3/S3Console.vue'

defineProps<{
  resourceId: string
  resourceName: string
}>()
</script>

<style scoped>
.ws-s3 {
  display: flex;
  flex-direction: column;
  height: 100%;
}
</style>
