<template>
  <div
    class="editor-block"
    :class="{ focused: focused }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <BlockToolbar
      :is-visible="isHovered || focused"
      @add="$emit('add-after')"
    />
    <HeadingBlock
      v-if="block.type === 'heading'"
      ref="headingRef"
      :block-id="block.id"
      :content="block.content"
      :level="block.level ?? 1"
      :focused="focused"
      :suppress-keyboard="suppressKeyboard"
      @update:content="(c) => $emit('update:content', c)"
      @enter-pressed="$emit('enter-pressed')"
      @backspace-empty="$emit('backspace-empty')"
      @adjust-level="(d) => $emit('adjust-level', d)"
      @focus="$emit('focus')"
      @blur="$emit('blur')"
    />
    <ParagraphBlock
      v-else-if="block.type === 'paragraph'"
      ref="paragraphRef"
      :block-id="block.id"
      :content="block.content"
      :focused="focused"
      :suppress-keyboard="suppressKeyboard"
      @update:content="(c) => $emit('update:content', c)"
      @enter-pressed="$emit('enter-pressed')"
      @backspace-empty="$emit('backspace-empty')"
      @focus="$emit('focus')"
      @blur="$emit('blur')"
    />
    <CodeBlock
      v-else-if="block.type === 'code'"
      :block-id="block.id"
      :content="block.content"
    />
    <CommandBlock
      v-else-if="block.type === 'command'"
      :block-id="block.id"
      :content="block.content"
      :resource-id="block.resourceId"
      :protocol="block.protocol"
      @update:resource-id="(id) => $emit('update:resource-id', id)"
      @update:protocol="(p) => $emit('update:protocol', p)"
      @update:content="(c) => $emit('update:content', c)"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { EditorBlock } from '@/api/notebook'
import BlockToolbar from './BlockToolbar.vue'
import HeadingBlock from './blocks/HeadingBlock.vue'
import ParagraphBlock from './blocks/ParagraphBlock.vue'
import CodeBlock from './blocks/CodeBlock.vue'
import CommandBlock from './blocks/CommandBlock.vue'

defineProps<{
  block: EditorBlock
  focused: boolean
  suppressKeyboard?: boolean
}>()

defineEmits<{
  'update:content': [content: string]
  'update:resource-id': [id: string]
  'update:protocol': [protocol: string]
  'enter-pressed': []
  'backspace-empty': []
  'adjust-level': [direction: 1 | -1]
  'add-after': []
  focus: []
  blur: []
}>()

const isHovered = ref(false)
const headingRef = ref<InstanceType<typeof HeadingBlock>>()
const paragraphRef = ref<InstanceType<typeof ParagraphBlock>>()

defineExpose({
  focus() {
    headingRef.value?.focus()
    paragraphRef.value?.focus()
  },
})
</script>

<style scoped>
.editor-block {
  position: relative;
  padding-left: 40px;
  transition: border-color var(--transition-fast);
}

.editor-block.focused {
  border-left: 2px solid var(--info);
  padding-left: 38px;
}
</style>
