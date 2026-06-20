<template>
  <SettingsSection>
    <template #header>{{ t('settings.terminal.title') }}</template>

    <!-- Font Size -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.fontSize') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.fontSizeDesc') }}</div>
      </div>
      <select class="form-select" :value="fontSize" @change="setFontSize">
        <option v-for="size in fontSizes" :key="size" :value="size">{{ size }}</option>
      </select>
    </div>
    <!-- Font Family -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.fontFamily') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.fontFamilyDesc') }}</div>
      </div>
      <select class="form-select" :value="fontFamily" @change="setFontFamily">
        <option v-for="f in fontFamilies" :key="f.value" :value="f.value">{{ f.label }}</option>
      </select>
    </div>
    <!-- Cursor Blink -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.cursorBlink') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.cursorBlinkDesc') }}</div>
      </div>
      <div class="settings-toggle" :class="{ active: cursorBlink }" @click="toggleCursorBlink"></div>
    </div>
    <!-- Keepalive -->
    <div class="settings-row">
      <div class="settings-row-info">
        <div class="settings-row-label">{{ t('settings.terminal.keepalive') }}</div>
        <div class="settings-row-desc">{{ t('settings.terminal.keepaliveDesc') }}</div>
      </div>
      <select class="form-select" :value="keepalive" @change="setKeepalive">
        <option v-for="k in keepaliveOptions" :key="k" :value="k">{{ k }}</option>
      </select>
    </div>
  </SettingsSection>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import SettingsSection from './SettingsSection.vue'

const { t } = useI18n()

// ── Terminal settings state ──
const fontSizes = [12, 13, 14, 15, 16]
const fontFamilies = [
  { value: 'JetBrains Mono', label: 'JetBrains Mono' },
  { value: 'Fira Code', label: 'Fira Code' },
  { value: 'Cascadia Code', label: 'Cascadia Code' },
  { value: 'Source Code Pro', label: 'Source Code Pro' },
]
const keepaliveOptions = [30, 60, 120]

const fontSize = ref(Number(localStorage.getItem('rex-term-font-size')) || 13)
const fontFamily = ref(localStorage.getItem('rex-term-font-family') || 'JetBrains Mono')
const cursorBlink = ref(localStorage.getItem('rex-term-cursor-blink') !== 'false')
const keepalive = ref(Number(localStorage.getItem('rex-term-keepalive')) || 60)

function setFontSize(e: Event) {
  fontSize.value = Number((e.target as HTMLSelectElement).value)
  localStorage.setItem('rex-term-font-size', String(fontSize.value))
}

function setFontFamily(e: Event) {
  fontFamily.value = (e.target as HTMLSelectElement).value
  localStorage.setItem('rex-term-font-family', fontFamily.value)
}

function toggleCursorBlink() {
  cursorBlink.value = !cursorBlink.value
  localStorage.setItem('rex-term-cursor-blink', String(cursorBlink.value))
}

function setKeepalive(e: Event) {
  keepalive.value = Number((e.target as HTMLSelectElement).value)
  localStorage.setItem('rex-term-keepalive', String(keepalive.value))
}
</script>
