<template>
  <div class="deploy-guide">
    <h3 class="guide-title">{{ t('agent.deploy') }}</h3>

    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="tab-btn"
        :class="{ active: activeTab === tab.key }"
        @click="activeTab = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>

    <div class="tab-content">
      <!-- Binary -->
      <div v-if="activeTab === 'binary'" class="code-block">
        <p class="code-desc">{{ t('agent.tokenNote') }}</p>
        <div class="code-line">
          <pre><code># Linux / macOS
chmod +x rex-agent
./rex-agent --worker \
  --config agent.yaml</code></pre>
        </div>
        <div class="code-line">
          <pre><code># agent.yaml 内容示例
server: http://your-hub:3000
token: YOUR_AGENT_TOKEN
name: my-agent
data_dir: ./agent-data</code></pre>
        </div>
      </div>

      <!-- Docker -->
      <div v-if="activeTab === 'docker'" class="code-block">
        <div class="code-line">
          <pre><code>docker run -d --name rex-agent \
  -e REX_SERVER=http://your-hub:3000 \
  -e REX_TOKEN=YOUR_AGENT_TOKEN \
  -e REX_NAME=my-agent \
  -v agent-data:/data \
  rex-agent:latest --worker</code></pre>
        </div>
      </div>

      <!-- Docker Compose -->
      <div v-if="activeTab === 'docker-compose'" class="code-block">
        <div class="code-line">
          <pre><code>version: '3.8'
services:
  agent:
    image: rex-agent:latest
    command: ["--worker"]
    environment:
      - REX_SERVER=http://your-hub:3000
      - REX_TOKEN=YOUR_AGENT_TOKEN
      - REX_NAME=my-agent
      - REX_DATA_DIR=/data
    volumes:
      - agent-data:/data

volumes:
  agent-data:</code></pre>
        </div>
      </div>

      <!-- Config File -->
      <div v-if="activeTab === 'config'" class="code-block">
        <div class="code-line">
          <pre><code># agent.yaml
server: http://your-hub:3000
token: YOUR_AGENT_TOKEN
name: my-agent
data_dir: /var/lib/rex-agent</code></pre>
        </div>
        <div class="code-line">
          <pre><code># 启动
./rex-agent --worker --config /etc/rex-agent/agent.yaml</code></pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const activeTab = ref('binary')

const tabs = computed(() => [
  { key: 'binary', label: t('agent.binary') },
  { key: 'docker', label: t('agent.docker') },
  { key: 'docker-compose', label: t('agent.dockerCompose') },
  { key: 'config', label: t('agent.configFile') },
])
</script>

<style scoped>
.deploy-guide {
  margin-top: var(--sp-2xl);
  padding-top: var(--sp-xl);
  border-top: 1px solid var(--border);
}

.guide-title {
  font-size: var(--fs-base);
  font-weight: 600;
  margin-bottom: var(--sp-lg);
}

.tabs {
  display: flex;
  gap: var(--sp-xs);
  margin-bottom: var(--sp-lg);
}

.tab-btn {
  padding: var(--sp-sm) var(--sp-md);
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.tab-btn:hover {
  border-color: var(--accent);
  color: var(--text);
}

.tab-btn.active {
  background: rgba(232, 145, 45, 0.1);
  border-color: var(--accent);
  color: var(--accent);
}

.tab-content {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--sp-lg);
}

.code-desc {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin-bottom: var(--sp-md);
}

.code-block {
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
}

.code-line {
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--sp-md);
  overflow-x: auto;
}

.code-line pre {
  margin: 0;
}

.code-line code {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--text);
  white-space: pre;
}
</style>
