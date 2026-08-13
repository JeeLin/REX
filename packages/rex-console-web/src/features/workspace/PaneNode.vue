<script setup lang="ts">
import { Splitpanes, Pane } from 'splitpanes'
import type { PaneNode } from '@/composables/usePaneLayout'
import PaneLeaf from './PaneLeaf.vue'
import 'splitpanes/dist/splitpanes.css'

defineProps<{ node: PaneNode }>()
</script>

<template>
  <!-- 叶子节点：直接渲染 pane 内容 -->
  <PaneLeaf v-if="node.direction === null" :leaf-id="node.id" />

  <!-- 容器节点：用自己的 direction 决定分栏方向，递归渲染子节点 -->
  <Splitpanes
    v-else
    :horizontal="node.direction === 'column'"
    class="ws-split"
  >
    <Pane
      v-for="child in node.children"
      :key="child.id"
      :size="child.size"
      :min-size="20"
    >
      <PaneNode :node="child" />
    </Pane>
  </Splitpanes>
</template>

<style scoped>
.ws-split {
  height: 100%;
}
</style>
