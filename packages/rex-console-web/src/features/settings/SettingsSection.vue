<template>
  <div class="settings-section">
    <div class="settings-section-header">
      <slot name="header" />
    </div>
    <div class="settings-section-body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
</script>

<style>
/* Shared settings section styles (not scoped — used by child components) */
.settings-section {
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  margin-bottom: var(--sp-lg);
  overflow: hidden;
  transition: border-color var(--transition-base), box-shadow var(--transition-base);
}

.settings-section:hover {
  border-color: var(--primary);
  box-shadow: 0 0 0 1px var(--primary-subtle);
}

.settings-section-header {
  padding: var(--sp-lg) var(--sp-xl);
  border-bottom: 1px solid var(--border-subtle);
  font-family: var(--font-code);
  font-weight: 700;
  font-size: var(--fs-md);
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.settings-section-body {
  padding: var(--sp-xl);
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-md) 0;
  border-bottom: 1px solid var(--border-subtle);
}

.settings-row:last-child {
  border-bottom: none;
}

.settings-row-info {
  flex: 1;
}

.settings-row-label {
  font-weight: 600;
  margin-bottom: 2px;
  color: var(--text-primary);
}

.settings-row-desc {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

/* Toggle switch */
.settings-toggle {
  width: 44px;
  height: 24px;
  background: var(--border-default);
  border-radius: 12px;
  position: relative;
  cursor: pointer;
  transition: background var(--transition-fast), box-shadow var(--transition-fast);
  flex-shrink: 0;
}

.settings-toggle.active {
  background: var(--primary);
  box-shadow: var(--glow-sm);
}

.settings-toggle::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--bg-surface);
  transition: transform var(--transition-fast);
}

.settings-toggle.active::after {
  transform: translateX(18px);
}

/* Select */
.form-select {
  padding: var(--sp-xs) var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-deep);
  color: var(--text-primary);
  font-family: var(--font-body);
  font-size: var(--fs-sm);
  outline: none;
  height: 28px;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath fill='%238B949E' d='M4 6L0 2h8z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 6px center;
  padding-right: 20px;
}

.form-select:focus {
  border-color: var(--accent);
}

@media (max-width: 767px) {
  .settings-row {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-md);
  }
}
</style>
