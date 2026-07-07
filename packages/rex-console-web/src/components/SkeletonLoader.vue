<template>
  <div class="skeleton-wrapper" :class="`skeleton-${variant}`">
    <!-- Card variant: stat cards + env/agent cards -->
    <template v-if="variant === 'card'">
      <div class="skeleton-stats-row">
        <div v-for="n in 4" :key="`stat-${n}`" class="skeleton-stat-card">
          <div class="skeleton-line skeleton-label"></div>
          <div class="skeleton-line skeleton-value"></div>
        </div>
      </div>
      <div class="skeleton-cards-grid">
        <div v-for="n in (count || 3)" :key="`card-${n}`" class="skeleton-card">
          <div class="skeleton-card-header">
            <div class="skeleton-line skeleton-title"></div>
            <div class="skeleton-badge"></div>
          </div>
          <div class="skeleton-line skeleton-text"></div>
          <div class="skeleton-line skeleton-text-short"></div>
        </div>
      </div>
    </template>

    <!-- List variant: env/agent list cards -->
    <template v-else-if="variant === 'list'">
      <div class="skeleton-cards-grid">
        <div v-for="n in (count || 4)" :key="`list-${n}`" class="skeleton-card">
          <div class="skeleton-card-header">
            <div class="skeleton-line skeleton-title"></div>
            <div class="skeleton-badge"></div>
          </div>
          <div class="skeleton-line skeleton-text"></div>
          <div class="skeleton-line skeleton-text-short"></div>
        </div>
      </div>
    </template>

    <!-- Table variant: table rows -->
    <template v-else>
      <div class="skeleton-table">
        <div v-for="n in (count || 5)" :key="`row-${n}`" class="skeleton-table-row">
          <div class="skeleton-line skeleton-cell"></div>
          <div class="skeleton-line skeleton-cell-short"></div>
          <div class="skeleton-line skeleton-cell"></div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  variant?: 'card' | 'list' | 'table'
  count?: number
}>()
</script>

<style scoped>
.skeleton-wrapper {
  padding: 16px;
}

/* ── Animations ── */
@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.skeleton-line {
  background: linear-gradient(
    90deg,
    var(--bg-elevated) 25%,
    var(--bg-hover) 50%,
    var(--bg-elevated) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
  border-radius: var(--radius-sm);
}

/* ── Stats Row ── */
.skeleton-stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

.skeleton-stat-card {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

/* ── Cards Grid ── */
.skeleton-cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.skeleton-card {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

.skeleton-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.skeleton-badge {
  width: 48px;
  height: 20px;
  border-radius: 12px;
  background: linear-gradient(
    90deg,
    var(--bg-elevated) 25%,
    var(--bg-hover) 50%,
    var(--bg-elevated) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}

/* ── Table ── */
.skeleton-table {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  overflow: hidden;
}

.skeleton-table-row {
  display: flex;
  gap: 16px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.skeleton-table-row:last-child {
  border-bottom: none;
}

/* ── Line Sizes ── */
.skeleton-label {
  height: 12px;
  width: 60%;
  margin-bottom: 8px;
}

.skeleton-value {
  height: 24px;
  width: 40%;
}

.skeleton-title {
  height: 16px;
  width: 50%;
}

.skeleton-text {
  height: 12px;
  width: 80%;
  margin-top: 8px;
}

.skeleton-text-short {
  height: 12px;
  width: 50%;
  margin-top: 8px;
}

.skeleton-cell {
  height: 14px;
  flex: 1;
}

.skeleton-cell-short {
  height: 14px;
  width: 100px;
}

/* ── Mobile ── */
@media (max-width: 767px) {
  .skeleton-stats-row {
    grid-template-columns: repeat(2, 1fr);
    gap: var(--sp-sm);
  }

  .skeleton-cards-grid {
    grid-template-columns: 1fr;
    gap: var(--sp-sm);
  }
}
</style>
