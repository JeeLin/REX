<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { askAi, generateSql } from './ai-api'

const { t } = useI18n()

const props = defineProps<{
  show: boolean
  sessionId: string
}>()

const emit = defineEmits<{
  close: []
  'update:selectedText': [text: string]
}>()

// Chat state
interface ChatMessage {
  id: number
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  sql?: string
}

const messages = ref<ChatMessage[]>([])
const userInput = ref('')
const selectedText = ref('')
const loading = ref(false)
const mode = ref<'ask' | 'generate'>('ask')
const chatContainer = ref<HTMLDivElement>()

// Auto-scroll to bottom when new messages arrive
watch(messages, async () => {
  await nextTick()
  if (chatContainer.value) {
    chatContainer.value.scrollTop = chatContainer.value.scrollHeight
  }
}, { deep: true })

// Clear messages when panel is closed
watch(() => props.show, (v) => {
  if (!v) {
    messages.value = []
    userInput.value = ''
    selectedText.value = ''
  }
})

function generateId(): number {
  return Date.now() + Math.random()
}

/**
 * Simple markdown renderer for AI responses
 * Handles: code blocks, bold, italic, headers, lists, blockquotes
 */
function renderMarkdown(text: string): string {
  let html = text
    // Escape HTML
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
  
  // Code blocks (must be before inline code)
  html = html.replace(/```(\w+)?\n([\s\S]*?)```/g, (_, lang, code) => {
    return `<pre class="ai-code-block"><code class="language-${lang || 'sql'}">${code.trim()}</code></pre>`
  })
  
  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code class="ai-inline-code">$1</code>')
  
  // Headers
  html = html.replace(/^### (.+)$/gm, '<h4 class="ai-heading">$1</h4>')
  html = html.replace(/^## (.+)$/gm, '<h3 class="ai-heading">$1</h3>')
  html = html.replace(/^# (.+)$/gm, '<h2 class="ai-heading">$1</h2>')
  
  // Bold and italic
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>')
  
  // Blockquotes
  html = html.replace(/^&gt; (.+)$/gm, '<blockquote class="ai-blockquote">$1</blockquote>')
  
  // Unordered lists
  html = html.replace(/^- (.+)$/gm, '<li class="ai-list-item">$1</li>')
  html = html.replace(/(<li class="ai-list-item">.*<\/li>\n?)+/g, (match) => {
    return `<ul class="ai-list">${match}</ul>`
  })
  
  // Ordered lists
  html = html.replace(/^\d+\. (.+)$/gm, '<li class="ai-list-item">$1</li>')
  
  // Line breaks
  html = html.replace(/\n/g, '<br>')
  
  return html
}

function setMode(newMode: 'ask' | 'generate') {
  mode.value = newMode
}

async function sendMessage() {
  const question = userInput.value.trim()
  if (!question && !selectedText.value) return

  const displayQuestion = selectedText.value 
    ? `${question || 'Analyze this SQL:'}\n\`\`\`\n${selectedText.value}\n\`\`\``
    : question

  // Add user message
  messages.value.push({
    id: generateId(),
    role: 'user',
    content: displayQuestion,
    timestamp: new Date()
  })

  userInput.value = ''
  loading.value = true

  try {
    const context = selectedText.value || undefined
    
    if (mode.value === 'generate') {
      const response = await generateSql(question, context)
      messages.value.push({
        id: generateId(),
        role: 'assistant',
        content: response.answer,
        timestamp: new Date(),
        sql: response.sql
      })
    } else {
      const response = await askAi(question, context)
      messages.value.push({
        id: generateId(),
        role: 'assistant',
        content: response,
        timestamp: new Date()
      })
    }
  } catch (error) {
    messages.value.push({
      id: generateId(),
      role: 'assistant',
      content: `**Error:** ${error instanceof Error ? error.message : 'Failed to get AI response'}`,
      timestamp: new Date()
    })
  } finally {
    loading.value = false
    selectedText.value = ''
  }
}

function copyResponse(content: string) {
  // Extract plain text from markdown
  const plainText = content
    .replace(/```[\s\S]*?```/g, (match) => match.replace(/```\w*\n?/g, '').trim())
    .replace(/[*_`#>]/g, '')
    .trim()
  
  navigator.clipboard?.writeText(plainText)
}

function copyCode(sql: string) {
  navigator.clipboard?.writeText(sql)
}

function insertSql(sql: string) {
  // Emit event to insert SQL into editor
  emit('update:selectedText', sql)
}

function clearChat() {
  messages.value = []
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="ai-panel">
      <div v-if="show" class="ai-panel-overlay" @click.self="emit('close')">
        <div class="ai-panel">
          <!-- Header -->
          <div class="ai-panel-header">
            <div class="ai-panel-title">
              <span class="ai-icon">🤖</span>
              <span>{{ t('ai.title') || 'AI Assistant' }}</span>
            </div>
            <div class="ai-panel-actions">
              <button 
                class="ai-action-btn"
                :title="t('ai.clearChat') || 'Clear chat'"
                @click="clearChat"
              >
                🗑️
              </button>
              <button 
                class="ai-close-btn"
                :title="t('ai.close') || 'Close'"
                @click="emit('close')"
              >
                ×
              </button>
            </div>
          </div>

          <!-- Mode Toggle -->
          <div class="ai-mode-toggle">
            <button 
              class="ai-mode-btn"
              :class="{ 'ai-mode-btn--active': mode === 'ask' }"
              @click="setMode('ask')"
            >
              💬 {{ t('ai.askMode') || 'Ask AI' }}
            </button>
            <button 
              class="ai-mode-btn"
              :class="{ 'ai-mode-btn--active': mode === 'generate' }"
              @click="setMode('generate')"
            >
              📝 {{ t('ai.generateMode') || 'Generate SQL' }}
            </button>
          </div>

          <!-- Selected Text Indicator -->
          <div v-if="selectedText" class="ai-selected-text">
            <span class="ai-selected-label">{{ t('ai.selectedText') || 'Selected:' }}</span>
            <code class="ai-selected-content">{{ selectedText.slice(0, 100) }}{{ selectedText.length > 100 ? '...' : '' }}</code>
            <button class="ai-selected-clear" @click="selectedText = ''">×</button>
          </div>

          <!-- Chat Messages -->
          <div ref="chatContainer" class="ai-chat-container">
            <div v-if="messages.length === 0" class="ai-empty-state">
              <div class="ai-empty-icon">🤖</div>
              <div class="ai-empty-title">{{ t('ai.welcomeTitle') || 'AI Assistant' }}</div>
              <div class="ai-empty-desc">
                {{ t('ai.welcomeDesc') || 'Ask questions about SQL, optimize queries, or generate new queries.' }}
              </div>
              <div class="ai-quick-actions">
                <button class="ai-quick-btn" @click="userInput = 'Explain this SQL query'; sendMessage()">
                  {{ t('ai.explainQuery') || 'Explain query' }}
                </button>
                <button class="ai-quick-btn" @click="userInput = 'Optimize this query for performance'; sendMessage()">
                  {{ t('ai.optimizeQuery') || 'Optimize query' }}
                </button>
                <button class="ai-quick-btn" @click="mode = 'generate'">
                  {{ t('ai.generateNew') || 'Generate new query' }}
                </button>
              </div>
            </div>

            <div 
              v-for="msg in messages" 
              :key="msg.id"
              class="ai-message"
              :class="`ai-message--${msg.role}`"
            >
              <div class="ai-message-avatar">
                {{ msg.role === 'user' ? '👤' : '🤖' }}
              </div>
              <div class="ai-message-content">
                <div 
                  class="ai-message-text"
                  v-html="renderMarkdown(msg.content)"
                />
                <div v-if="msg.sql" class="ai-message-sql">
                  <div class="ai-sql-header">
                    <span>{{ t('ai.generatedSql') || 'Generated SQL' }}</span>
                    <div class="ai-sql-actions">
                      <button 
                        class="ai-sql-btn"
                        :title="t('ai.copyCode') || 'Copy code'"
                        @click="copyCode(msg.sql!)"
                      >
                        📋
                      </button>
                      <button 
                        class="ai-sql-btn ai-sql-btn--primary"
                        :title="t('ai.insertToEditor') || 'Insert to editor'"
                        @click="insertSql(msg.sql!)"
                      >
                        {{ t('ai.insert') || 'Insert' }}
                      </button>
                    </div>
                  </div>
                  <pre class="ai-sql-code"><code>{{ msg.sql }}</code></pre>
                </div>
                <div class="ai-message-actions">
                  <button 
                    class="ai-msg-action-btn"
                    :title="t('ai.copyResponse') || 'Copy response'"
                    @click="copyResponse(msg.content)"
                  >
                    📋 {{ t('ai.copy') || 'Copy' }}
                  </button>
                </div>
              </div>
            </div>

            <div v-if="loading" class="ai-message ai-message--assistant">
              <div class="ai-message-avatar">🤖</div>
              <div class="ai-message-content">
                <div class="ai-loading">
                  <div class="ai-loading-dot" />
                  <div class="ai-loading-dot" />
                  <div class="ai-loading-dot" />
                </div>
              </div>
            </div>
          </div>

          <!-- Input Area -->
          <div class="ai-input-area">
            <textarea
              v-model="userInput"
              class="ai-input"
              :placeholder="mode === 'ask' 
                ? (t('ai.askPlaceholder') || 'Ask a question about SQL...') 
                : (t('ai.generatePlaceholder') || 'Describe the SQL query you want...')"
              rows="2"
              @keydown="handleKeydown"
            />
            <button 
              class="ai-send-btn"
              :disabled="loading || (!userInput.trim() && !selectedText)"
              :title="t('ai.send') || 'Send'"
              @click="sendMessage"
            >
              <span v-if="loading" class="ai-send-loading">⏳</span>
              <span v-else>➤</span>
            </button>
          </div>

          <!-- Keyboard Hint -->
          <div class="ai-keyboard-hint">
            {{ t('ai.keyboardHint') || 'Press Enter to send, Shift+Enter for new line' }}
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
/* Panel Overlay */
.ai-panel-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(4px);
  z-index: 1000;
  display: flex;
  justify-content: flex-end;
}

/* Panel Container */
.ai-panel {
  width: 420px;
  max-width: 90vw;
  height: 100%;
  background: var(--bg-surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  box-shadow: -10px 0 40px rgba(0, 0, 0, 0.4);
}

/* Header */
.ai-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
  background: var(--bg-elevated);
}

.ai-panel-title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
}

.ai-icon {
  font-size: var(--text-lg);
}

.ai-panel-actions {
  display: flex;
  gap: var(--space-1);
}

.ai-action-btn,
.ai-close-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  transition: all var(--transition);
}

.ai-action-btn:hover,
.ai-close-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.ai-close-btn {
  font-size: var(--text-lg);
}

/* Mode Toggle */
.ai-mode-toggle {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}

.ai-mode-btn {
  flex: 1;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--transition);
}

.ai-mode-btn:hover {
  border-color: var(--accent);
  color: var(--text-primary);
}

.ai-mode-btn--active {
  background: rgba(232, 145, 45, 0.1);
  border-color: var(--accent);
  color: var(--accent);
}

/* Selected Text */
.ai-selected-text {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);
  background: var(--bg-deep);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
}

.ai-selected-label {
  color: var(--text-muted);
  flex-shrink: 0;
}

.ai-selected-content {
  flex: 1;
  font-family: var(--font-mono);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ai-selected-clear {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: var(--radius-sm);
}

.ai-selected-clear:hover {
  color: var(--danger);
}

/* Chat Container */
.ai-chat-container {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

/* Empty State */
.ai-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  gap: var(--space-3);
}

.ai-empty-icon {
  font-size: 48px;
  opacity: 0.5;
}

.ai-empty-title {
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.ai-empty-desc {
  font-size: var(--text-sm);
  color: var(--text-muted);
  max-width: 280px;
}

.ai-quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  justify-content: center;
  margin-top: var(--space-3);
}

.ai-quick-btn {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition);
}

.ai-quick-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* Messages */
.ai-message {
  display: flex;
  gap: var(--space-3);
}

.ai-message--user {
  flex-direction: row-reverse;
}

.ai-message-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--bg-deep);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--text-sm);
  flex-shrink: 0;
}

.ai-message--user .ai-message-avatar {
  background: var(--accent);
}

.ai-message-content {
  flex: 1;
  min-width: 0;
}

.ai-message-text {
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius);
  font-size: var(--text-sm);
  line-height: 1.6;
  color: var(--text-primary);
}

.ai-message--user .ai-message-text {
  background: var(--accent);
  color: var(--text-on-accent);
}

/* Markdown Styles */
.ai-message-text :deep(.ai-code-block) {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-3);
  margin: var(--space-2) 0;
  overflow-x: auto;
}

.ai-message-text :deep(.ai-code-block code) {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-primary);
}

.ai-message-text :deep(.ai-inline-code) {
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 2px 4px;
  font-family: var(--font-mono);
  font-size: 0.9em;
}

.ai-message-text :deep(.ai-heading) {
  margin: var(--space-2) 0 var(--space-1);
  font-weight: 600;
  color: var(--accent);
}

.ai-message-text :deep(.ai-blockquote) {
  border-left: 3px solid var(--accent);
  padding-left: var(--space-3);
  margin: var(--space-2) 0;
  color: var(--text-muted);
  font-style: italic;
}

.ai-message-text :deep(.ai-list) {
  margin: var(--space-2) 0;
  padding-left: var(--space-4);
}

.ai-message-text :deep(.ai-list-item) {
  margin: var(--space-1) 0;
}

/* SQL Block */
.ai-message-sql {
  margin-top: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.ai-sql-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-deep);
  border-bottom: 1px solid var(--border);
  font-size: var(--text-xs);
  color: var(--text-muted);
}

.ai-sql-actions {
  display: flex;
  gap: var(--space-1);
}

.ai-sql-btn {
  padding: var(--space-1) var(--space-2);
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition);
}

.ai-sql-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.ai-sql-btn--primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--text-on-accent);
}

.ai-sql-btn--primary:hover {
  opacity: 0.9;
}

.ai-sql-code {
  margin: 0;
  padding: var(--space-3);
  background: var(--bg-deep);
  overflow-x: auto;
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--text-primary);
}

/* Message Actions */
.ai-message-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.ai-msg-action-btn {
  padding: var(--space-1) var(--space-2);
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: var(--text-xs);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--transition);
}

.ai-msg-action-btn:hover {
  color: var(--accent);
  background: var(--bg-hover);
}

/* Loading Animation */
.ai-loading {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-3);
  background: var(--bg-deep);
  border-radius: var(--radius);
}

.ai-loading-dot {
  width: 8px;
  height: 8px;
  background: var(--accent);
  border-radius: 50%;
  animation: ai-bounce 1.4s infinite ease-in-out;
}

.ai-loading-dot:nth-child(1) {
  animation-delay: -0.32s;
}

.ai-loading-dot:nth-child(2) {
  animation-delay: -0.16s;
}

@keyframes ai-bounce {
  0%, 80%, 100% {
    transform: scale(0);
    opacity: 0.5;
  }
  40% {
    transform: scale(1);
    opacity: 1;
  }
}

/* Input Area */
.ai-input-area {
  display: flex;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border);
  background: var(--bg-elevated);
}

.ai-input {
  flex: 1;
  padding: var(--space-3);
  background: var(--bg-deep);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  resize: none;
  outline: none;
  transition: border-color var(--transition);
}

.ai-input:focus {
  border-color: var(--accent);
}

.ai-input::placeholder {
  color: var(--text-muted);
}

.ai-send-btn {
  padding: var(--space-3) var(--space-4);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-on-accent);
  font-size: var(--text-lg);
  cursor: pointer;
  transition: opacity var(--transition);
  min-width: 48px;
}

.ai-send-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.ai-send-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.ai-send-loading {
  animation: ai-spin 1s linear infinite;
}

@keyframes ai-spin {
  to {
    transform: rotate(360deg);
  }
}

/* Keyboard Hint */
.ai-keyboard-hint {
  padding: var(--space-2) var(--space-4);
  text-align: center;
  font-size: var(--text-xs);
  color: var(--text-muted);
  background: var(--bg-elevated);
  border-top: 1px solid var(--border);
}

/* Panel Transitions */
.ai-panel-enter-active,
.ai-panel-leave-active {
  transition: opacity 0.3s ease;
}

.ai-panel-enter-active .ai-panel,
.ai-panel-leave-active .ai-panel {
  transition: transform 0.3s ease;
}

.ai-panel-enter-from,
.ai-panel-leave-to {
  opacity: 0;
}

.ai-panel-enter-from .ai-panel,
.ai-panel-leave-to .ai-panel {
  transform: translateX(100%);
}
</style>
