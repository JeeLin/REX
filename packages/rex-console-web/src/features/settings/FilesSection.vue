<template>
  <SettingsSection>
    <template #header>{{ t('settings.files.title') }}</template>

    <!-- Max Concurrent Transfers -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.files.maxConcurrent') }}</div>
        <div class="settings-row-desc">{{ t('settings.files.maxConcurrentDesc') }}</div>
      </div>
      <div class="concurrency-control">
        <input
          type="range"
          min="1"
          max="10"
          :value="maxConcurrent"
          @input="onConcurrencyChange"
          class="concurrency-slider"
        />
        <span class="concurrency-value">{{ maxConcurrent }}</span>
      </div>
    </div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import SettingsSection from './SettingsSection.vue'
import { getTransferConcurrency, setTransferConcurrency } from '@/api/transfer'

const { t } = useI18n()
const maxConcurrent = ref(3)

onMounted(async () => {
  try {
    maxConcurrent.value = await getTransferConcurrency()
  } catch {
    // keep default
  }
})

async function onConcurrencyChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value)
  maxConcurrent.value = val
  try {
    await setTransferConcurrency(val)
  } catch {
    // revert on failure
  }
}
</script>

<style scoped>
.concurrency-control {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.concurrency-slider {
  width: 120px;
  accent-color: var(--accent);
}

.concurrency-value {
  min-width: 24px;
  text-align: center;
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-base);
  color: var(--accent);
}
</style>
