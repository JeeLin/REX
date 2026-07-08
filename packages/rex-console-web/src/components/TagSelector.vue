<template>
  <div class="tag-selector" ref="rootRef">
    <!-- Selected tags -->
    <div class="tag-selected" v-if="modelValue.length">
      <span
        v-for="tag in selectedTags"
        :key="tag.id"
        class="tag-pill"
        :style="{ '--tag-color': tag.color }"
      >
        <span class="tag-dot" />
        {{ tag.name }}
        <button class="tag-remove" @click="removeTag(tag.id)">×</button>
      </span>
    </div>

    <!-- Dropdown trigger + search -->
    <div class="tag-input-wrap">
      <input
        ref="inputRef"
        v-model="query"
        class="form-input tag-input"
        :placeholder="modelValue.length ? t('resource.tagMore') : t('resource.tagsPlaceholder')"
        @focus="open = true"
        @input="open = true"
      />

      <!-- Dropdown -->
      <div v-if="open" class="tag-dropdown">
        <!-- Existing tags -->
        <div class="tag-dropdown-list" v-if="filteredTags.length">
          <button
            v-for="tag in filteredTags"
            :key="tag.id"
            class="tag-option"
            :class="{ active: modelValue.includes(tag.id) }"
            @click="toggleTag(tag.id)"
          >
            <span class="tag-dot" :style="{ background: tag.color }" />
            <span class="tag-option-name">{{ tag.name }}</span>
            <span v-if="modelValue.includes(tag.id)" class="tag-check">✓</span>
          </button>
        </div>
        <div v-else-if="!query.trim()" class="tag-dropdown-empty">
          {{ t('resource.tagsEmpty') }}
        </div>

        <!-- Create new tag -->
        <div v-if="query.trim() && !existingTagNames.has(query.trim())" class="tag-create">
          <div class="tag-create-row">
            <span class="tag-create-label">{{ t('resource.tagCreate') }} "{{ query.trim() }}"</span>
          </div>
          <div class="tag-color-picker">
            <button
              v-for="color in presetColors"
              :key="color"
              class="tag-color-btn"
              :class="{ active: newTagColor === color }"
              :style="{ background: color }"
              @click="newTagColor = color"
            />
          </div>
          <button class="btn btn-ghost btn-sm tag-create-btn" @click="handleCreate">
            {{ t('resource.tagAdd') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { listTags, createTag, type Tag } from '@/api/tags'

const { t } = useI18n()

const props = defineProps<{
  modelValue: string[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string[]]
}>()

const presetColors = [
  '#58A6FF', '#3FB950', '#E8912D', '#F85149',
  '#D2A8FF', '#79C0FF', '#56D364', '#FFA657',
  '#FF7B72', '#F778BA', '#A5D6FF', '#B392F0',
]

const rootRef = ref<HTMLElement>()
const inputRef = ref<HTMLInputElement>()
const open = ref(false)
const query = ref('')
const allTags = ref<Tag[]>([])
const newTagColor = ref('#58A6FF')

const existingTagNames = computed(() => new Set(allTags.value.map(t => t.name)))

const selectedTags = computed(() =>
  allTags.value.filter(t => props.modelValue.includes(t.id))
)

const filteredTags = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return allTags.value
  return allTags.value.filter(t => t.name.toLowerCase().includes(q))
})

function toggleTag(id: string) {
  const current = props.modelValue
  if (current.includes(id)) {
    emit('update:modelValue', current.filter(i => i !== id))
  } else {
    emit('update:modelValue', [...current, id])
  }
  query.value = ''
}

function removeTag(id: string) {
  emit('update:modelValue', props.modelValue.filter(i => i !== id))
}

async function handleCreate() {
  const name = query.value.trim()
  if (!name) return
  try {
    const tag = await createTag({ name, color: newTagColor.value })
    allTags.value.push(tag)
    emit('update:modelValue', [...props.modelValue, tag.id])
    query.value = ''
  } catch {
    // silent
  }
}

async function loadTags() {
  try {
    allTags.value = await listTags()
  } catch {
    allTags.value = []
  }
}

function handleClickOutside(e: MouseEvent) {
  if (rootRef.value && !rootRef.value.contains(e.target as Node)) {
    open.value = false
  }
}

onMounted(() => {
  loadTags()
  document.addEventListener('click', handleClickOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.tag-selector {
  position: relative;
}

.tag-selected {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 8px;
}

.tag-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  background: color-mix(in srgb, var(--tag-color) 15%, transparent);
  color: var(--tag-color);
  border: 1px solid color-mix(in srgb, var(--tag-color) 30%, transparent);
}

.tag-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--tag-color, currentColor);
  flex-shrink: 0;
}

.tag-remove {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  padding: 0 2px;
  opacity: 0.7;
}

.tag-remove:hover {
  opacity: 1;
}

.tag-input-wrap {
  position: relative;
}

.tag-input {
  width: 100%;
  box-sizing: border-box;
}

.tag-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 4px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow-lg);
  z-index: 100;
  max-height: 240px;
  overflow-y: auto;
}

.tag-dropdown-list {
  padding: 4px;
}

.tag-option {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 10px;
  border: none;
  background: none;
  color: var(--text);
  cursor: pointer;
  border-radius: 6px;
  font-size: 13px;
  text-align: left;
}

.tag-option:hover {
  background: var(--bg-hover);
}

.tag-option.active {
  background: var(--bg-active);
}

.tag-option-name {
  flex: 1;
}

.tag-check {
  color: var(--success);
  font-size: 14px;
}

.tag-dropdown-empty {
  padding: 12px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
}

.tag-create {
  padding: 8px 10px;
  border-top: 1px solid var(--border);
}

.tag-create-row {
  margin-bottom: 6px;
}

.tag-create-label {
  font-size: 13px;
  color: var(--text-muted);
}

.tag-color-picker {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  margin-bottom: 6px;
}

.tag-color-btn {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: transform 0.1s;
}

.tag-color-btn:hover {
  transform: scale(1.15);
}

.tag-color-btn.active {
  border-color: var(--text);
  transform: scale(1.15);
}

.tag-create-btn {
  width: 100%;
}
</style>
