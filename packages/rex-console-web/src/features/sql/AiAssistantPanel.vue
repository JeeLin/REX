<template>
  <div class="ai-panel" :class="{ 'ai-panel-open': isOpen }">
    <div class="ai-panel-header">
      <div class="ai-panel-title">✦ AI 助手</div>
      <button class="ai-panel-close" @click="closePanel">×</button>
    </div>

    <!-- 风险提示 -->
    <div class="ai-warning">
      ⚠ AI 生成的 SQL 可能存在逻辑错误或性能问题。请务必先在测试环境验证。
    </div>

    <!-- 上下文栏 -->
    <div class="ai-context-bar" v-if="context">
      <span
        v-for="(item, index) in contextItems"
        :key="index"
        class="ai-context-item"
      >
        {{ item.icon }} {{ item.text }}
      </span>
    </div>

    <!-- 消息列表 -->
    <div class="ai-messages" ref="messagesContainer">
      <ai-message
        v-for="message in messages"
        :key="message.id"
        :message="message"
        @copy-sql="copySqlToEditor"
      />
    </div>

    <!-- 快捷操作 -->
    <div class="ai-quick-actions">
      <button
        class="ai-quick-btn"
        @click="quickAction('generate')"
        :disabled="isStreaming"
      >
        生成 SQL
      </button>
      <button
        class="ai-quick-btn"
        @click="quickAction('analyze')"
        :disabled="isStreaming"
      >
        分析慢查询
      </button>
      <button
        class="ai-quick-btn"
        @click="quickAction('relations')"
        :disabled="isStreaming"
      >
        表关系
      </button>
    </div>

    <!-- 输入区域 -->
    <div class="ai-input-area">
      <textarea
        class="ai-input"
        v-model="inputValue"
        @keydown.enter.exact="sendMessage"
        placeholder="向 AI 提问..."
        :disabled="isStreaming"
      ></textarea>
      <div class="ai-input-actions">
        <button
          class="ai-send-btn"
          @click="sendMessage"
          :disabled="!inputValue.trim() || isStreaming"
        >
          {{ isStreaming ? "停止" : "发送" }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from "vue";
import {
  useAiChat,
  type AiConversationMessage,
} from "@/features/sql/useAiChat";
import { useI18n } from "vue-i18n";
import AiMessage from "./AiMessage.vue";

const { t } = useI18n();

// Props
const props = defineProps<{
  context: {
    database?: string;
    tables?: string[];
    dialect?: string;
  } | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (e: "update:visible"): void;
  (e: "close"): void;
}>();

// State
const isOpen = ref(false);
const inputValue = ref("");
const isStreaming = ref(false);
const messages = ref<AiConversationMessage[]>([]);

// Composables
const {
  messages: aiMessages,
  input: aiInput,
  isStreaming: aiIsStreaming,
  loadConfig,
  sendMessage: aiSendMessage,
  quickAction: aiQuickAction,
  stopStreaming: aiStopStreaming,
} = useAiChat(
  props.context ?? {
    database: undefined,
    tables: [],
    dialect: undefined,
  },
);

// Sync with composable state
watch(aiMessages, (newMessages) => {
  messages.value = newMessages;
});
watch(aiIsStreaming, (newVal) => {
  isStreaming.value = newVal;
});
watch(aiInput, (newVal) => {
  inputValue.value = newVal;
});

// Panel visibility
watch(
  () => props.visible,
  (newVisible) => {
    isOpen.value = newVisible;
    if (newVisible) {
      loadConfig();
      // Focus input after panel opens
      nextTick(() => {
        const textarea = document.querySelector(
          ".ai-input",
        ) as HTMLTextAreaElement;
        textarea?.focus();
      });
    }
  },
);

// Computed properties
const contextItems = computed(() => {
  const items: Array<{ icon: string; text: string }> = [];
  if (props.context?.database) {
    items.push({ icon: "🗄", text: props.context.database });
  }
  if (props.context?.tables?.length) {
    items.push({ icon: "📋", text: props.context.tables.join(", ") });
  }
  if (props.context?.dialect) {
    items.push({ icon: "🔤", text: props.context.dialect });
  }
  return items;
});

// Methods
function closePanel() {
  isOpen.value = false;
  emit("update:visible", false);
  emit("close");
}

async function handleSendMessage() {
  if (!inputValue.value.trim() || isStreaming.value) return;

  await aiSendMessage(inputValue.value);
  inputValue.value = "";
}

function handleQuickAction(action: "generate" | "analyze" | "relations") {
  aiQuickAction(action);
}

function handleStopStreaming() {
  aiStopStreaming();
}

// Lifecycle
onMounted(() => {
  loadConfig();

  // Global keyboard shortcuts
  const handleKeydown = (e: KeyboardEvent) => {
    // Ctrl+Shift+A to toggle panel
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "a") {
      e.preventDefault();
      emit("update:visible", !props.visible);
    }

    // Escape to close panel
    if (e.key === "Escape" && isOpen.value) {
      closePanel();
    }
  };

  window.addEventListener("keydown", handleKeydown);

  // Cleanup
  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
});
</script>

<style scoped>
.ai-panel {
  position: fixed;
  right: 0;
  top: 0;
  bottom: 0;
  width: 360px;
  max-width: 90vw;
  background: var(--bg-panel);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  transform: translateX(100%);
  transition: transform 0.3s ease;
  z-index: 1000;
  overflow: hidden;
  font-size: var(--fs-sm);
}

.ai-panel-open {
  transform: translateX(0);
}

.ai-panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border);
}

.ai-panel-title {
  font-weight: 600;
  color: var(--text-primary);
  font-size: var(--fs-base);
}

.ai-panel-close {
  background: transparent;
  border: none;
  font-size: var(--fs-lg);
  line-height: 1;
  color: var(--text-muted);
  cursor: pointer;
  padding: var(--sp-xs);
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.ai-panel-close:hover {
  color: var(--text-danger);
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
}

.ai-warning {
  padding: var(--sp-sm) var(--sp-md);
  background: var(--bg-warning);
  color: var(--text-warning);
  font-size: var(--fs-xs);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
}

.ai-context-bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-xs);
  padding: var(--sp-sm) var(--sp-md);
  border-bottom: 1px solid var(--border);
  background: var(--bg-input);
}

.ai-context-item {
  display: flex;
  align-items: center;
  gap: var(--sp-xxs);
  font-size: var(--fs-xs);
  color: var(--text-muted);
  background: var(--bg-elevated);
  padding: var(--sp-xs) var(--sp-sm);
  border-radius: var(--radius-sm);
}

.ai-messages {
  flex: 1;
  overflow-y: auto;
  padding: var(--sp-sm);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.ai-input-area {
  padding: var(--sp-sm) var(--sp-md);
  border-top: 1px solid var(--border);
  background: var(--bg-panel);
}

.ai-input {
  width: 100%;
  min-height: 80px;
  resize: vertical;
  padding: var(--sp-sm);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--fs-sm);
  transition: border-color 0.2s;
}

.ai-input:focus {
  outline: none;
  border-color: var(--accent-primary);
}

.ai-input:disabled {
  background: var(--bg-disabled);
  color: var(--text-disabled);
  cursor: not-allowed;
}

.ai-input-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-xs);
  margin-top: var(--sp-xs);
}

.ai-send-btn {
  padding: var(--sp-xs) var(--sp-md);
  background: var(--accent-primary);
  color: var(--text-invert);
  border: none;
  border-radius: var(--radius-sm);
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.2s;
}

.ai-send-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.ai-send-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.ai-quick-actions {
  display: flex;
  gap: var(--sp-xs);
  padding: var(--sp-sm) var(--sp-md);
  border-top: 1px solid var(--border);
}

.ai-quick-btn {
  flex: 1;
  padding: var(--sp-sm);
  border: 1px solid var(--border);
  background: var(--bg-input);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  font-size: var(--fs-xs);
  cursor: pointer;
  transition: all 0.2s;
}

.ai-quick-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: var(--border-hover);
}

.ai-quick-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Scrollbar styling */
.ai-messages::-webkit-scrollbar {
  width: 8px;
}

.ai-messages::-webkit-scrollbar-track {
  background: var(--bg-panel);
}

.ai-messages::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}

.ai-messages::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
}
</style>
