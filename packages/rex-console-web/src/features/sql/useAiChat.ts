import { ref, computed } from "vue";
import {
  getAiConfig,
  sendAiMessage,
  type ChatMessage,
  type AiContext,
  type AiConfigResponse,
} from "@/api/ai";

export interface AiConversationMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  streaming?: boolean;
}

export function useAiChat(context: AiContext) {
  const messages = ref<AiConversationMessage[]>([]);
  const input = ref("");
  const isStreaming = ref(false);
  const config = ref<AiConfigResponse | null>(null);

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

    try {
      const apiMessages: ChatMessage[] = messages.value
        .filter((m) => m.role === "user" || m.role === "assistant")
        .map((m) => ({ role: m.role, content: m.content }));

      const response = await sendAiMessage(apiMessages, context);

      // Handle SSE stream
      const reader = (response as any).getReader();
      const decoder = new TextDecoder();

      while (true) {
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
                }
              } catch {
                // Plain text token
                assistantMsg.content += data;
              }
            }
          }
        }
      }
    } catch (error) {
      console.error("AI chat failed:", error);
      assistantMsg.streaming = false;
      assistantMsg.content = "AI 请求失败，请检查配置或重试";
    } finally {
      isStreaming.value = false;
    }
  }

  function quickAction(action: "generate" | "analyze" | "relations") {
    const prompts = {
      generate: "根据当前数据库的表结构，生成一个常用查询示例",
      analyze: "分析当前查询的性能瓶颈，并提供优化建议",
      relations: "分析当前数据库的表关系（外键、关联），生成 ER 图描述",
    };
    sendMessage(prompts[action]);
  }

  function stopStreaming() {
    isStreaming.value = false;
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
