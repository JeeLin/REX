<template>
  <SettingsSection>
    <template #header>{{ t('settings.terminal.title') }}</template>

    <!-- Font Size -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.fontSize') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.fontSizeDesc') }}</div>
      </div>
      <select class="form-select" :value="terminalSettings.fontSize" @change="setFontSize">
        <option v-for="size in fontSizes" :key="size" :value="size">{{ size }}</option>
      </select>
    </div>
    <!-- Font Family -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.fontFamily') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.fontFamilyDesc') }}</div>
      </div>
      <select class="form-select" :value="terminalSettings.fontFamily" @change="setFontFamily">
        <option v-for="f in fontFamilies" :key="f.value" :value="f.value">{{ f.label }}</option>
      </select>
    </div>
    <!-- Cursor Blink -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.cursorBlink') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.cursorBlinkDesc') }}</div>
      </div>
      <div class="settings-toggle" :class="{ active: terminalSettings.cursorBlink }" @click="toggleCursorBlink"></div>
    </div>
    <!-- Keepalive -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.keepalive') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.keepaliveDesc') }}</div>
      </div>
      <select class="form-select" :value="terminalSettings.keepalive" @change="setKeepalive">
        <option v-for="k in keepaliveOptions" :key="k" :value="k">{{ k }}</option>
      </select>
    </div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import SettingsSection from './SettingsSection.vue'
import { terminalSettings, updateTerminalSetting } from '@/stores/settings'

const { t } = useI18n()

const fontSizes = [12, 13, 14, 15, 16]
const fontFamilies = [
  { value: 'JetBrains Mono', label: 'JetBrains Mono' },
  { value: 'Fira Code', label: 'Fira Code' },
  { value: 'Cascadia Code', label: 'Cascadia Code' },
  { value: 'Source Code Pro', label: 'Source Code Pro' },
]
const keepaliveOptions = [30, 60, 120]

function setFontSize(e: Event) {
  updateTerminalSetting('fontSize', Number((e.target as HTMLSelectElement).value))
}

function setFontFamily(e: Event) {
  updateTerminalSetting('fontFamily', (e.target as HTMLSelectElement).value)
}

function toggleCursorBlink() {
  updateTerminalSetting('cursorBlink', !terminalSettings.cursorBlink)
}

function setKeepalive(e: Event) {
  updateTerminalSetting('keepalive', Number((e.target as HTMLSelectElement).value))
}
</script>
