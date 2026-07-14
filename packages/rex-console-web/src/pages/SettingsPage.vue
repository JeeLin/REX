<script setup lang="ts">
import { ref } from 'vue'
import Card from '@/components/ui/Card.vue'
import Button from '@/components/ui/Button.vue'
import { useI18n } from 'vue-i18n'

const { t, locale } = useI18n()
const theme = ref<'dark' | 'light'>('dark')
const terminalFont = ref('JetBrains Mono')

function saveSettings() {
  document.documentElement.dataset.theme = theme.value === 'light' ? 'light' : undefined
}
</script>

<template>
  <div class="settings">
    <header class="page-header">
      <h1 class="page-title">Settings</h1>
      <Button variant="primary" size="sm" @click="saveSettings">Save Changes</Button>
    </header>

    <div class="settings-grid">
      <Card title="Appearance">
        <div class="setting-row">
          <div class="setting-label">Theme</div>
          <div class="setting-control">
            <label class="radio">
              <input type="radio" value="dark" v-model="theme" />
              <span>Dark</span>
            </label>
            <label class="radio">
              <input type="radio" value="light" v-model="theme" />
              <span>Light</span>
            </label>
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-label">Language</div>
          <div class="setting-control">
            <select v-model="locale" class="select">
              <option value="zh">中文</option>
              <option value="en">English</option>
            </select>
          </div>
        </div>
      </Card>

      <Card title="Terminal">
        <div class="setting-row">
          <div class="setting-label">Font Family</div>
          <div class="setting-control">
            <select v-model="terminalFont" class="select">
              <option value="JetBrains Mono">JetBrains Mono</option>
              <option value="Fira Code">Fira Code</option>
              <option value="Cascadia Code">Cascadia Code</option>
            </select>
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-label">Font Size</div>
          <div class="setting-control">
            <input type="range" min="11" max="20" value="13" class="range" />
            <span class="mono muted">13px</span>
          </div>
        </div>
      </Card>
    </div>
  </div>
</template>

<style scoped>
.settings {
  max-width: 900px;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-6);
}
.page-title {
  font-size: var(--text-xl);
  font-weight: 600;
  color: var(--text-primary);
}
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: var(--space-4);
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) 0;
  border-bottom: 1px solid var(--border-subtle);
}
.setting-row:last-child {
  border-bottom: none;
}
.setting-label {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.setting-control {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.radio {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--text-sm);
  cursor: pointer;
}
.select {
  height: 32px;
  padding: 0 var(--space-2);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  cursor: pointer;
}
.select:focus {
  border-color: var(--accent);
}
.range {
  width: 120px;
}
</style>
