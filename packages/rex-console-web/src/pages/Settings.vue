<template>
  <div class="settings-page">
    <div class="settings-layout">
      <ProfileSection />
      <AppearanceSection />
      <TerminalSection />
      <SecuritySection />
      <TlsSection />
      <BackupSection />
      <UpdateSection />
      <LoadingSpinner v-if="loading" :text="t('common.loading')" />
      <template v-else>
        <div class="version-info">
          <div class="ver">REX Hub {{ health?.version }}</div>
          <div class="version-sub">自托管 · 开源</div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import ProfileSection from '@/features/settings/ProfileSection.vue'
import AppearanceSection from '@/features/settings/AppearanceSection.vue'
import TerminalSection from '@/features/settings/TerminalSection.vue'
import SecuritySection from '@/features/settings/SecuritySection.vue'
import TlsSection from '@/features/settings/TlsSection.vue'
import BackupSection from '@/features/settings/BackupSection.vue'
import UpdateSection from '@/features/settings/UpdateSection.vue'
import { fetchHealth, HealthStatus } from '@/api/health'

const { t } = useI18n()

const health = ref<HealthStatus | null>(null)
const loading = ref(true)

onMounted(async () => {
  try {
    health.value = await fetchHealth()
  } catch (error) {
    console.error('Failed to fetch health:', error)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.settings-page {
  padding: 0;
}

.settings-layout {
  max-width: 640px;
}

.version-info {
  text-align: center;
  padding: var(--sp-2xl) 0;
  color: var(--text-muted);
  font-size: var(--fs-sm);
}

.version-info .ver {
  font-family: var(--font-mono);
  color: var(--text-secondary);
  font-weight: 600;
}

.version-sub {
  margin-top: var(--sp-xs);
}
</style>
