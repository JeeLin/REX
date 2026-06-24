<template>
  <div class="ai-message" :class="[message.role, { streaming: message.streaming }]">
    <div class="ai-message-avatar">
      {{ message.role === 'user' ? '👤' : '🤖' }}
    </div>
    <div class="ai-message-content">
      <div class="ai-message-header">
        <span class="ai-message-role">
          {{ message.role === 'user' ? '您' : 'AI 助手' }}
        </span>
        <span v-if="message.streaming" class="ai-message-streaming">●</span>
      </div>

      <div v-html="renderContent(message.content)" class="ai-message-body"></div>

      <!-- Copy button for code blocks -->
      <template v-if="!message.streaming && hasCodeBlocks(message.content)">
        <div class="ai-message-actions">
          <button
            v-for="(block, index) in extractCodeBlocks(message.content)"
            :key="index"
            class="ai-copy-btn"
            @click="copySqlToEditor(block)"
            title="复制 SQL 到编辑器"
          >
            📋 复制
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  message: {
    id: string
    role: 'user' | 'assistant'
    content: string
    streaming?: boolean
  }
}>()

const emit = defineEmits<{
  (e: 'copy-sql', sql: string): void
}>()

// Simple markdown renderer (inline code and code blocks)
function renderContent(content: string): string {
  let html = content
    // Escape HTML first
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')

  // Code blocks
  html = html.replace(/```sql([\s\S]*?)```/g, '<pre><code class="language-sql">$1</code></pre>')
  html = html.replace(/```([\s\S]*?)```/g, '<pre><code>$1</code></pre>')

  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>')

  // Bold
  html = html.replace(/\*\*([^\s]+)\*\*/g, '<strong>$1</strong>')

  // Newlines to br
  html = html.replace(/\n/g, '<br>')

  return html
}

function hasCodeBlocks(content: string): boolean {
  return content.includes('```') || content.includes('`')
}

function extractCodeBlocks(content: string): string[] {
  const blocks: string[] = []
  const sqlRegex = /```sql([\s\S]*?)```/g
  let match
  while ((match = sqlRegex.exec(content)) !== null) {
    blocks.push(match[1].trim())
  }
  // Also extract inline code if no blocks
  if (blocks.length === 0) {
    const inlineRegex = /`([^`]+)`/g
    while ((match = inlineRegex.exec(content)) !== null) {
      if (looksLikeSql(match[1])) {
        blocks.push(match[1])
      }
    }
  }
  return blocks
}

function looksLikeSql(text: string): boolean {
  const sqlKeywords = ['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'CREATE', 'DROP', 'ALTER', 'FROM', 'WHERE', 'JOIN']
  const upperText = text.toUpperCase()
  return sqlKeywords.some(kw => upperText.includes(kw))
}

function copySqlToEditor(sql: string) {
  emit('copy-sql', sql)
}
</script>

<style scoped>
.ai-message {
  display: flex;
  gap: var(--sp-sm);
}

.ai-message.user {
  justify-content: flex-end;
}

.ai-message.assistant {
  justify-content: flex-start;
}

.ai-message-avatar {
  font-size: var(--fs-base);
}

.ai-message-content {
  max-width: 80%;
}

.ai-message-header {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  margin-bottom: var(--sp-xxs);
}

.ai-message-role {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.ai-message.streaming .ai-message-streaming {
  display: inline-block;
  width: 8px;
  height: 8px;
  background: var(--text-primary);
  border-radius: 50%;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.ai-message-body {
  font-size: var(--fs-sm);
  line-height: 1.5;
  color: var(--text-primary);
}

.ai-message-body pre {
  background: var(--bg-elevated);
  padding: var(--sp-sm);
  border-radius: var(--radius-sm);
  overflow-x: auto;
  margin: var(--sp-xs) 0;
}

.ai-message-body code {
  background: var(--bg-elevated);
  padding: 2px 6px;
  border-radius: var(--radius-xs);
  font-family: var(--font-mono);
  font-size: 0.9em;
}

.ai-message-body pre code {
  background: transparent;
  padding: 0;
}

.ai-message-actions {
  margin-top: var(--sp-xs);
}

.ai-copy-btn {
  padding: var(--sp-xxs) var(--sp-xs);
  font-size: var(--fs-xs);
  background: var(--accent-primary);
  color: var(--text-invert);
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
}
</style>