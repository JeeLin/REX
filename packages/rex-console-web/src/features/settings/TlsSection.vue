<template>
  <SettingsSection>
    <template #header>{{ t('settings.tls.title') }}</template>

    <div v-if="loading" class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-desc">{{ t('settings.tls.loading') }}</div>
      </div>
    </div>

    <template v-else-if="tlsStatus">
      <!-- TLS 模式 -->
      <div class="settings-row">
        <div class="settings-row-info">
          <div class="settings-row-label">{{ t('settings.tls.mode') }}</div>
          <div class="settings-row-desc">{{ modeDescription }}</div>
        </div>
        <span class="tls-mode-badge" :class="modeBadgeClass">{{ modeLabel }}</span>
      </div>

      <!-- 域名/IP（ACME 模式） -->
      <div v-if="tlsStatus.domain" class="settings-row">
        <div class="settings-row-info">
          <div class="settings-row-label">{{ t('settings.tls.domain') }}</div>
          <div class="settings-row-desc">{{ tlsStatus.domain }}</div>
        </div>
      </div>

      <!-- 证书状态 -->
      <div class="settings-row">
        <div class="settings-row-info">
          <div class="settings-row-label">{{ t('settings.tls.certStatus') }}</div>
          <div class="settings-row-desc">{{ tlsStatus.cert_ready ? t('settings.tls.certReady') : t('settings.tls.certPending') }}</div>
        </div>
        <span class="tls-status-dot" :class="{ ready: tlsStatus.cert_ready }"></span>
      </div>

      <!-- 颁发者 -->
      <div v-if="tlsStatus.cert_issuer" class="settings-row">
        <div class="settings-row-info">
          <div class="settings-row-label">{{ t('settings.tls.issuer') }}</div>
          <div class="settings-row-desc">{{ tlsStatus.cert_issuer }}</div>
        </div>
      </div>

      <!-- 端口提示 -->
      <div v-if="tlsStatus.port_80_required" class="settings-row">
        <div class="settings-row-info">
          <div class="settings-row-label">{{ t('settings.tls.port80Required') }}</div>
          <div class="settings-row-desc">{{ t('settings.tls.port80RequiredDesc') }}</div>
        </div>
      </div>
    </template>

    <div v-else class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-desc">{{ t('settings.tls.error') }}</div>
      </div>
    </div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import SettingsSection from './SettingsSection.vue'
import { getTlsStatus, type TlsStatus } from '@/api/tls'

const { t } = useI18n()

const loading = ref(true)
const tlsStatus = ref<TlsStatus | null>(null)

const modeLabel = computed(() => {
  if (!tlsStatus.value) return ''
  const labels: Record<string, string> = {
    'manual': t('settings.tls.modeManual'),
    'acme-domain': t('settings.tls.modeAcmeDomain'),
    'acme-ip': t('settings.tls.modeAcmeIp'),
    'self-signed': t('settings.tls.modeSelfSigned'),
    'none': t('settings.tls.modeNone'),
  }
  return labels[tlsStatus.value.mode] || tlsStatus.value.mode
})

const modeDescription = computed(() => {
  if (!tlsStatus.value) return ''
  const descs: Record<string, string> = {
    'manual': t('settings.tls.modeManualDesc'),
    'acme-domain': t('settings.tls.modeAcmeDomainDesc'),
    'acme-ip': t('settings.tls.modeAcmeIpDesc'),
    'self-signed': t('settings.tls.modeSelfSignedDesc'),
    'none': t('settings.tls.modeNoneDesc'),
  }
  return descs[tlsStatus.value.mode] || ''
})

const modeBadgeClass = computed(() => {
  if (!tlsStatus.value) return ''
  const classes: Record<string, string> = {
    'manual': 'badge-green',
    'acme-domain': 'badge-blue',
    'acme-ip': 'badge-blue',
    'self-signed': 'badge-yellow',
    'none': 'badge-gray',
  }
  return classes[tlsStatus.value.mode] || ''
})

onMounted(async () => {
  try {
    tlsStatus.value = await getTlsStatus()
  } catch (e) {
    console.error('Failed to load TLS status:', e)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.tls-mode-badge {
  font-size: var(--fs-xs);
  font-family: var(--font-mono);
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-weight: 500;
}

.badge-green {
  background: rgba(46, 160, 67, 0.15);
  color: #3fb950;
}

.badge-blue {
  background: rgba(56, 132, 244, 0.15);
  color: #58a6ff;
}

.badge-yellow {
  background: rgba(210, 153, 34, 0.15);
  color: #d29922;
}

.badge-gray {
  background: rgba(139, 148, 158, 0.15);
  color: #8b949e;
}

.tls-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
  flex-shrink: 0;
}

.tls-status-dot.ready {
  background: #3fb950;
}
</style>
