import { ref, computed } from "vue";
import {
  getAiConfig,
  sendAiMessage,
  type ChatMessage,
  type AiContext,
  type AiConfigResponse,
} from "@/api/ai";
import { useI18n } from "vue-i18n";

export interface AiConversationMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  streaming?: boolean;
}

export function useAiChat(context: AiContext) {
  const { t } = useI18n();
  const messages = ref<AiConversationMessage[]>([]);
  const input = ref("");
  const isStreaming = ref(false);
  const config = ref<AiConfigResponse | null>(null);
  const abortController = ref<AbortController | null>(null);

  const isOpen = computed(() => messages.value.length > 0 || isStreaming.value);

  async function loadConfig() {
    try {
      config.value = await getAiConfig();
    } catch {
      config.value = null;
    }
  }

  async function sendMessage(content: string) {
    if (!content.trim() || isStreaming.value) return;

    const userMsg: AiConversationMessage = {
      id: `user_${Date.now()}`,
      role: "user",
      content,
    };
    messages.value.push(userMsg);

    const assistantMsg: AiConversationMessage = {
      id: `assistant_${Date.now()}`,
      role: "assistant",
      content: "",
      streaming: true,
    };
    messages.value.push(assistantMsg);

    isStreaming.value = true;
    input.value = "";

    // Create abort controller for stop functionality
    abortController.value = new AbortController();

    try {
      const apiMessages: ChatMessage[] = messages.value
        .filter((m) => m.role === "user" || m.role === "assistant")
        .map((m) => ({ role: m.role, content: m.content }));

      const response = await sendAiMessage(apiMessages, context);

      // Handle SSE stream from Response
      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error("No response body");
      }
      const decoder = new TextDecoder();

      while (true) {
        if (!isStreaming.value) break;

        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value, { stream: true });
        const lines = chunk.split("\n");

        for (const line of lines) {
          if (line.startsWith("data: ")) {
            const data = line.slice(6);
            if (data.startsWith("[DONE]")) {
              assistantMsg.streaming = false;
            } else {
              try {
                const event = JSON.parse(data);
                if (event.type === "token") {
                  assistantMsg.content += event.content;
                } else if (event.type === "error") {
                  console.error("AI error:", event.content);
                  assistantMsg.streaming = false;
                  assistantMsg.content = t('sql.ai.requestFailed') + event.content;
                }
              } catch {
                // Plain text token (OpenAI format)
                assistantMsg.content += data;
              }
            }
          }
        }
      }
    } catch (error) {
      console.error("AI chat failed:", error);
      assistantMsg.streaming = false;
      assistantMsg.content = t('sql.ai.requestError');
    } finally {
      isStreaming.value = false;
      abortController.value = null;
    }
  }

  function quickAction(action: "generate" | "analyze" | "relations") {
    const prompts = {
      generate: t('sql.ai.presetGenerate'),
      analyze: t('sql.ai.presetAnalyze'),
      relations: t('sql.ai.presetRelations'),
    };
    void sendMessage(prompts[action]);
  }

  function stopStreaming() {
    isStreaming.value = false;
    if (abortController.value) {
      abortController.value.abort();
    }
  }

  return {
    messages,
    input,
    isStreaming,
    config,
    loadConfig,
    sendMessage,
    quickAction,
    stopStreaming,
  };
}