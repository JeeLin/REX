<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const emit = defineEmits<{
  'send-command': [command: string]
  'create-new-tab': [command: string, label: string]
}>()

const isOpen = ref(false)
const selectedContainer = ref<string | null>(null)
const menuRef = ref<HTMLElement>()
const buttonRef = ref<HTMLElement>()

const dropdownPosition = computed(() => {
  if (!buttonRef.value) return { top: '0px', left: '0px' }
  const rect = buttonRef.value.getBoundingClientRect()
  return {
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
  }
})

function handleClickAway(e: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    isOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickAway)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickAway)
})

function runCommand(command: string) {
  emit('send-command', command)
  isOpen.value = false
}

function promptAndRun(commandFn: (name: string) => void) {
  if (selectedContainer.value) {
    commandFn(selectedContainer.value)
    selectedContainer.value = null
  }
  isOpen.value = false
}

function handleExec() {
  if (selectedContainer.value) {
    const cmd = `docker exec -it ${selectedContainer.value} /bin/bash`
    emit('create-new-tab', cmd, `docker exec ${selectedContainer.value}`)
    selectedContainer.value = null
  }
  isOpen.value = false
}

function handleCustom() {
  const cmd = prompt(t('terminal.dockerCustomPrompt'))
  if (cmd && cmd.trim()) {
    runCommand(`docker ${cmd.trim()}`)
  }
  isOpen.value = false
}

function handleRefresh() {
  // Re-send Ctrl+L to clear and refresh the screen
  runCommand('\x0c')
}

function confirmRemove(name: string): boolean {
  return confirm(t('terminal.dockerRemoveConfirm'))
}

function selectContainer(name: string) {
  if (selectedContainer.value === name) {
    selectedContainer.value = null
  } else {
    selectedContainer.value = name
  }
}

function containerListCmd() {
  runCommand('docker ps --format table {{.Names}}\t{{.Status}}\t{{.Ports}}')
}
const imageListCmd = () => runCommand('docker images --format table {{.Repository}}\t{{.Tag}}\t{{.Size}}')
</script>

<template>
  <span ref="menuRef" class="docker-menu-wrapper">
    <button
      ref="buttonRef"
      class="wt-btn"
      title="Docker"
      @click.stop="isOpen = !isOpen"
    >
      🐳
    </button>
    <Teleport to="body">
      <Transition name="menu">
        <div
          v-if="isOpen"
          class="docker-dropdown"
          :style="dropdownPosition"
          @click.stop
        >
          <div class="docker-dropdown-section-title">Containers</div>
          <div class="docker-dropdown-item docker-dropdown-item--muted" @click="containerListCmd">
            📦 {{ t('terminal.dockerContainerList') }}
          </div>
          <div v-if="selectedContainer" class="docker-dropdown-item docker-dropdown-item--selected">
            🎯 {{ selectedContainer }}
            <span class="docker-dropdown-item-hint">Selected</span>
          </div>
          <div class="docker-dropdown-item docker-dropdown-item--hint" @click="selectedContainer = null">
            💡 {{ t('terminal.dockerSelectHint') || 'Click container name in terminal to select' }}
          </div>
          <div class="docker-dropdown-item" @click="imageListCmd">
            🖼️ {{ t('terminal.dockerImageList') }}
          </div>
          <div class="docker-dropdown-item" @click="handleRefresh">
            🔄 {{ t('common.refresh') }}
          </div>
          <div class="docker-dropdown-separator" />
          <div class="docker-dropdown-item" @click="promptAndRun((name) => runCommand(`docker start ${name}`))">
            ▶️ {{ t('terminal.dockerStartContainer') }}
          </div>
          <div class="docker-dropdown-item" @click="promptAndRun((name) => runCommand(`docker stop ${name}`))">
            ⏹️ {{ t('terminal.dockerStopContainer') }}
          </div>
          <div class="docker-dropdown-item docker-dropdown-item--danger" @click="promptAndRun((name) => { if (confirmRemove(name)) runCommand(`docker rm -f ${name}`) })">
            🗑️ {{ t('terminal.dockerRemoveContainer') }}
          </div>
          <div class="docker-dropdown-item" @click="promptAndRun((name) => runCommand(`docker logs -f --tail 100 ${name}`))">
            📝 {{ t('terminal.dockerViewLogs') }}
          </div>
          <div class="docker-dropdown-item" @click="handleExec">
            💻 {{ t('terminal.dockerExec') }}
          </div>
          <div class="docker-dropdown-separator" />
          <div class="docker-dropdown-item" @click="handleCustom">
            📟 {{ t('terminal.dockerCustomCommand') }}
          </div>
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<style scoped>
.docker-menu-wrapper {
  position: relative;
  display: inline-flex;
}

.docker-dropdown {
  position: fixed;
  min-width: 220px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  z-index: 100;
  padding: var(--space-1) 0;
}

.docker-dropdown-item {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-primary);
  cursor: pointer;
  transition: background var(--transition);
  white-space: nowrap;
}

.docker-dropdown-item:hover {
  background: var(--bg-hover);
}

.docker-dropdown-item--danger {
  color: var(--danger);
}

.docker-dropdown-item--danger:hover {
  background: rgba(248, 81, 73, 0.15);
}

.docker-dropdown-separator {
  height: 1px;
  background: var(--border);
  margin: var(--space-1) 0;
}

.menu-enter-active,
.menu-leave-active {
  transition: opacity var(--transition);
}

.docker-dropdown-section-title {
  padding: var(--space-2) var(--space-3) 2px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.docker-dropdown-item--selected {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success);
  font-weight: 500;
}

.docker-dropdown-item--hint {
  font-size: 11px;
  color: var(--text-muted);
  font-style: italic;
}

.docker-dropdown-item-hint {
  margin-left: auto;
  font-size: 10px;
  opacity: 0.7;
}

.docker-dropdown-item--muted {
  opacity: 0.7;
}

.menu-enter-from,
.menu-leave-to {
  opacity: 0;
}
</style>
