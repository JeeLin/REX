<template>
  <div class="deploy-guide">
    <div class="setup-header">
      <div class="setup-title">{{ t('agent.setupTitle') }}</div>
      <div class="setup-subtitle">{{ t('agent.setupSubtitle') }}</div>
    </div>

    <div class="setup-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="setup-tab"
        :class="{ active: activeTab === tab.key }"
        @click="activeTab = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>

    <!-- Binary Tab -->
    <div v-if="activeTab === 'binary'" class="setup-content">
      <div class="setup-step">
        <div class="step-number">1</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step1Download') }}</div>
          <div class="step-desc">{{ t('agent.step1DownloadDesc') }}</div>
          <div class="download-grid">
            <a v-for="dl in downloads" :key="dl.label" :href="dl.href"
               class="btn btn-ghost btn-sm download-btn" target="_blank">
              ⬇ {{ dl.label }}
            </a>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">2</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step2Token') }}</div>
          <div class="step-desc">{{ t('agent.step2TokenDesc') }}</div>
          <div v-if="agentToken" class="token-box">
            <span class="token-label">Token</span>
            <span class="token">{{ agentToken }}</span>
            <button class="btn btn-ghost btn-sm" @click="copyCmd('token')">
              {{ copyLabels.token }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">3</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step3Start') }}</div>
          <div class="step-desc">{{ t('agent.step3StartDesc') }}</div>
          <div class="code-block">
            <pre><code>./rex-agent \
  <span class="code-flag">--server</span> <span class="code-string">https://hub.example.com</span> \
  <span class="code-flag">--token</span>  <span class="code-string">YOUR_TOKEN</span> \
  <span class="code-flag">--name</span>   <span class="code-string">"My Agent"</span></code></pre>
            <button class="copy-btn" @click="copyCmd('startBinary')">
              {{ copyLabels.startBinary }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">4</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step4Confirm') }}</div>
          <div class="step-desc">{{ t('agent.step4ConfirmDesc') }}</div>
        </div>
      </div>
    </div>

    <!-- Docker Tab -->
    <div v-if="activeTab === 'docker'" class="setup-content">
      <div class="setup-step">
        <div class="step-number">1</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step1PullImage') }}</div>
          <div class="code-block">
            <pre><code>docker pull rexhub/agent:latest</code></pre>
            <button class="copy-btn" @click="copyCmd('pullImage')">
              {{ copyLabels.pullImage }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">2</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step2StartContainer') }}</div>
          <div class="step-desc">{{ t('agent.step2StartContainerDesc') }}</div>
          <div class="code-block">
            <pre><code>docker run -d --name rex-agent \
  -e REX_SERVER=https://hub.example.com \
  -e REX_TOKEN=YOUR_TOKEN \
  -e REX_NAME="My Agent" \
  -v agent-data:/data \
  rexhub/agent:latest --worker</code></pre>
            <button class="copy-btn" @click="copyCmd('dockerRun')">
              {{ copyLabels.dockerRun }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">3</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step4Confirm') }}</div>
          <div class="step-desc">{{ t('agent.step4ConfirmDesc') }}</div>
        </div>
      </div>
    </div>

    <!-- Docker Compose Tab -->
    <div v-if="activeTab === 'docker-compose'" class="setup-content">
      <div class="setup-step">
        <div class="step-number">1</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step1CreateConfig') }}</div>
          <div class="step-desc">{{ t('agent.step1CreateConfigDesc') }}</div>
          <div class="code-block">
            <pre><code>version: '3.8'
services:
  agent:
    image: rexhub/agent:latest
    command: ["--worker"]
    environment:
      - REX_SERVER=https://hub.example.com
      - REX_TOKEN=YOUR_TOKEN
      - REX_NAME=My Agent
      - REX_DATA_DIR=/data
    volumes:
      - agent-data:/data

volumes:
  agent-data:</code></pre>
            <button class="copy-btn" @click="copyCmd('composeYml')">
              {{ copyLabels.composeYml }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">2</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step2StartService') }}</div>
          <div class="step-desc">{{ t('agent.step2StartServiceDesc') }}</div>
          <div class="code-block">
            <pre><code>docker compose up -d</code></pre>
            <button class="copy-btn" @click="copyCmd('composeUp')">
              {{ copyLabels.composeUp }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">3</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step4Confirm') }}</div>
          <div class="step-desc">{{ t('agent.step4ConfirmDesc') }}</div>
        </div>
      </div>
    </div>

    <!-- Config File Tab -->
    <div v-if="activeTab === 'config'" class="setup-content">
      <div class="setup-step">
        <div class="step-number">1</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step1ConfigFile') }}</div>
          <div class="step-desc">{{ t('agent.step1ConfigFileDesc') }}</div>
          <div class="code-block">
            <pre><code><span class="code-comment"># agent.yaml</span>
server: https://hub.example.com
token: YOUR_TOKEN
name: My Agent
data_dir: /var/lib/rex-agent</code></pre>
            <button class="copy-btn" @click="copyCmd('configYaml')">
              {{ copyLabels.configYaml }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">2</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step2StartBinary') }}</div>
          <div class="step-desc">{{ t('agent.step2StartBinaryDesc') }}</div>
          <div class="code-block">
            <pre><code>./rex-agent --config agent.yaml</code></pre>
            <button class="copy-btn" @click="copyCmd('startConfig')">
              {{ copyLabels.startConfig }}
            </button>
          </div>
        </div>
      </div>

      <div class="setup-step">
        <div class="step-number">3</div>
        <div class="step-content">
          <div class="step-title">{{ t('agent.step4Confirm') }}</div>
          <div class="step-desc">{{ t('agent.step4ConfirmDesc') }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

defineProps<{ agentToken?: string }>()

const { t } = useI18n()
const activeTab = ref('binary')
const copiedKey = ref('')

const tabs = computed(() => [
  { key: 'binary', label: t('agent.binary') },
  { key: 'docker', label: t('agent.docker') },
  { key: 'docker-compose', label: t('agent.dockerCompose') },
  { key: 'config', label: t('agent.configFile') },
])

const downloads = [
  { label: 'linux-amd64', href: '/api/agent/download?os=linux&arch=amd64' },
  { label: 'linux-arm64', href: '/api/agent/download?os=linux&arch=arm64' },
  { label: 'linux-armv7l', href: '/api/agent/download?os=linux&arch=armv7l' },
  { label: 'darwin-arm64', href: '/api/agent/download?os=darwin&arch=arm64' },
  { label: 'darwin-amd64', href: '/api/agent/download?os=darwin&arch=amd64' },
  { label: 'windows-amd64', href: '/api/agent/download?os=windows&arch=amd64' },
]

const CMD_MAP: Record<string, string> = {
  token: '',
  startBinary: `./rex-agent \\
  --server https://hub.example.com \\
  --token  YOUR_TOKEN \\
  --name   "My Agent"`,
  pullImage: 'docker pull rexhub/agent:latest',
  dockerRun: `docker run -d --name rex-agent \\
  -e REX_SERVER=https://hub.example.com \\
  -e REX_TOKEN=YOUR_TOKEN \\
  -e REX_NAME="My Agent" \\
  -v agent-data:/data \\
  rexhub/agent:latest --worker`,
  composeYml: `version: '3.8'
services:
  agent:
    image: rexhub/agent:latest
    command: ["--worker"]
    environment:
      - REX_SERVER=https://hub.example.com
      - REX_TOKEN=YOUR_TOKEN
      - REX_NAME=My Agent
      - REX_DATA_DIR=/data
    volumes:
      - agent-data:/data

volumes:
  agent-data:`,
  composeUp: 'docker compose up -d',
  configYaml: `# agent.yaml
server: https://hub.example.com
token: YOUR_TOKEN
name: My Agent
data_dir: /var/lib/rex-agent`,
  startConfig: './rex-agent --config agent.yaml',
}

const copyLabels = computed(() => {
  const labels: Record<string, string> = {}
  for (const key of Object.keys(CMD_MAP)) {
    labels[key] = copiedKey.value === key ? t('agent.copied') : t('agent.copy')
  }
  labels.token = copiedKey.value === 'token' ? t('agent.copied') : t('agent.copy')
  return labels
})

function copyCmd(key: string) {
  const text = CMD_MAP[key] || ''
  navigator.clipboard.writeText(text)
  copiedKey.value = key
  setTimeout(() => { if (copiedKey.value === key) copiedKey.value = '' }, 2000)
}
</script>

<style scoped>
.deploy-guide {
  margin-top: var(--sp-2xl);
  padding-top: var(--sp-xl);
  border-top: 1px solid var(--border);
}

.setup-header {
  margin-bottom: var(--sp-lg);
}

.setup-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: var(--sp-xs);
}

.setup-subtitle {
  font-size: var(--fs-sm);
  color: var(--text-muted);
}

.setup-tabs {
  display: flex;
  gap: var(--sp-xs);
  margin-bottom: var(--sp-lg);
}

.setup-tab {
  padding: var(--sp-sm) var(--sp-md);
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--fs-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.setup-tab:hover {
  border-color: var(--accent);
  color: var(--text);
}

.setup-tab.active {
  background: rgba(232, 145, 45, 0.1);
  border-color: var(--accent);
  color: var(--accent);
}

.setup-content {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
}

.setup-step {
  display: flex;
  gap: var(--sp-md);
}

.step-number {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--accent);
  color: #000;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--fs-sm);
  font-weight: 700;
  flex-shrink: 0;
  font-family: var(--font-mono);
}

.step-content {
  flex: 1;
  min-width: 0;
}

.step-title {
  font-weight: 600;
  font-size: var(--fs-sm);
  margin-bottom: 4px;
}

.step-desc {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin-bottom: var(--sp-md);
  line-height: 1.5;
}

.download-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-sm);
  margin-bottom: var(--sp-md);
}

.download-btn {
  font-family: var(--font-mono) !important;
  font-size: var(--fs-xs) !important;
}

.token-box {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--sp-sm) var(--sp-md);
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}

.token-label {
  color: var(--text-muted);
  flex-shrink: 0;
}

.token {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
}

.code-block {
  position: relative;
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--sp-md);
  overflow-x: auto;
}

.code-block pre {
  margin: 0;
}

.code-block code {
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  color: var(--text);
  white-space: pre;
}

.code-comment { color: var(--text-muted); }
.code-flag { color: var(--accent); }
.code-string { color: var(--success); }

.copy-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 2px 8px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: 11px;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.copy-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}

@media (max-width: 767px) {
  .setup-tabs {
    flex-wrap: wrap;
  }

  .download-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-sm);
  }

  .token-box {
    flex-wrap: wrap;
  }

  .code-block {
    font-size: var(--fs-xs);
  }

  .code-block code {
    font-size: var(--fs-xs);
  }
}
</style>
