<script setup lang="ts" generic="T">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'

const props = defineProps<{
  items: T[]
  itemHeight: number
  buffer?: number
}>()

const containerRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const containerHeight = ref(0)

const buffer = computed(() => props.buffer ?? 5)

const totalHeight = computed(() => props.items.length * props.itemHeight)

const startIndex = computed(() =>
  Math.max(0, Math.floor(scrollTop.value / props.itemHeight) - buffer.value),
)

const endIndex = computed(() =>
  Math.min(
    props.items.length,
    Math.ceil((scrollTop.value + containerHeight.value) / props.itemHeight) + buffer.value,
  ),
)

const visibleItems = computed(() =>
  props.items.slice(startIndex.value, endIndex.value).map((item, i) => ({
    item,
    index: startIndex.value + i,
  })),
)

const offsetY = computed(() => startIndex.value * props.itemHeight)

function onScroll() {
  scrollTop.value = containerRef.value?.scrollTop ?? 0
}

function updateHeight() {
  containerHeight.value = containerRef.value?.clientHeight ?? 0
}

onMounted(() => {
  updateHeight()
  window.addEventListener('resize', updateHeight)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', updateHeight)
})

watch(
  () => props.items.length,
  () => updateHeight(),
)
</script>

<template>
  <div ref="containerRef" class="virtual-list" @scroll="onScroll">
    <div class="virtual-list-spacer" :style="{ height: totalHeight + 'px' }">
      <div class="virtual-list-content" :style="{ transform: `translateY(${offsetY}px)` }">
        <div
          v-for="entry in visibleItems"
          :key="entry.index"
          class="virtual-list-item"
          :style="{ height: itemHeight + 'px' }"
        >
          <slot :item="entry.item" :index="entry.index" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.virtual-list {
  overflow-y: auto;
  height: 100%;
}
.virtual-list-spacer {
  position: relative;
}
.virtual-list-content {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
}
</style>
