<template>
  <div v-if="visible" class="modal-overlay" @click.self="close">
    <div class="modal-content">
      <div class="modal-header">
        <span>{{ t('env.edit') }}</span>
        <button @click="close">×</button>
      </div>

      <div class="modal-body">
        <!-- Loading -->
        <div v-if="loading" class="loading-state">
          <p>{{ t('common.loading') }}</p>
        </div>

        <template v-else>
          <div class="form-group">
            <label class="form-label">{{ t('env.name') }}</label>
            <input v-model="form.name" class="form-input" :placeholder="t('env.namePlaceholder')" required />
          </div>

          <div class="form-group">
            <label class="form-label">{{ t('env.description') }}</label>
            <textarea v-model="form.description" class="form-input" :placeholder="t('env.descriptionPlaceholder')" rows="3"></textarea>
          </div>
        </template>
      </div>

      <div class="modal-footer">
        <button class="btn btn-ghost" @click="close">{{ t('common.cancel') }}</button>
        <button class="btn btn-primary" :disabled="loading || saving || !canSubmit" @click="submitUpdate">
          {{ saving ? t('common.saving') : t('env.save') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { updateEnvironment, getEnvironment, type Environment } from '@/api/env'
import { useSidebar } from '@/composables/useSidebar'

const props = defineProps<{
  visible: boolean
  envId: string
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>();

const { t } = useI18n()
const { fetchEnvs: refetchEnvs } = useSidebar()

const loading = ref(false)
const saving = ref(false)

const form = reactive({
  name: '',
  description: null as string | null,
})

watch(() => props.visible, async (v) => {
  if (!v || !props.envId) return
  loading.value = true
  try {
    const env = await getEnvironment(props.envId)
    form.name = env.name
    form.description = env.description || null
  } catch {
    emit('update:visible', false)
  } finally {
    loading.value = false
  }
})

function buildUpdateData() {
  return {
    name: form.name,
    description: form.description
  }
}

const canSubmit = computed(() => {
  return !!form.name.trim()
})

function close() {
  emit('update:visible', false)
}

async function submitUpdate() {
  saving.value = true
  try {
    await updateEnvironment(props.envId, buildUpdateData())
    refetchEnvs()
    close()
  } catch {
    // 错误处理可以在这里添加
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--bg-panel);
  border-radius: 8px;
  width: 90%;
  max-width: 480px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.modal-header span { font-weight: 600; font-size: 16px; color: var(--text-primary); }

.modal-header button {
  background: transparent; border: none; font-size: 24px; cursor: pointer;
  color: var(--text-secondary); width: 32px; height: 32px;
  display: flex; align-items: center; justify-content: center;
}

.modal-header button:hover { color: var(--text-primary); background: var(--bg-hover); border-radius: 4px; }

.modal-body {
  flex: 1;
  overflow: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid var(--border);
}

.form-row { display: flex; gap: 12px; }
.flex-1 { flex: 1; }
.flex-2 { flex: 2; }

.auth-toggle {
  display: flex; gap: 0; border: 1px solid var(--border); border-radius: 6px; overflow: hidden;
}

.auth-btn {
  flex: 1; padding: 8px 12px; background: var(--bg-deep); border: none;
  color: var(--text-secondary); font-size: 13px; cursor: pointer; transition: all 0.15s;
}

.auth-btn.active { background: var(--accent); color: #000; font-weight: 600; }

.loading-state { text-align: center; padding: 40px; color: var(--text-muted); }

@media (max-width: 767px) {
  .modal-content { width: 95%; max-height: 85vh; }
  .form-row { flex-direction: column; }
  .modal-body input,
  .modal-body textarea { font-size: 16px; min-height: 44px; }
}
</style>