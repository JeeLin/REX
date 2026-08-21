<script setup lang="ts">
import { ref } from 'vue'

withDefaults(defineProps<{
  src?: string
  size?: 'sm' | 'md' | 'lg'
  fallback?: string
}>(), { src: '', size: 'md', fallback: '?' })

const imgError = ref(false)
</script>

<template>
  <div class="avatar" :class="`avatar--${size}`">
    <img v-if="src" :src="src" class="avatar-img" @error="imgError = true" />
    <span v-if="!src || imgError" class="avatar-fallback">{{ fallback }}</span>
  </div>
</template>

<style scoped>
.avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-weight: 600;
  overflow: hidden;
  flex-shrink: 0;
}
.avatar--sm { width: 24px; height: 24px; font-size: 10px; }
.avatar--md { width: 32px; height: 32px; font-size: 12px; }
.avatar--lg { width: 48px; height: 48px; font-size: 16px; }
.avatar-img { width: 100%; height: 100%; object-fit: cover; }
</style>
