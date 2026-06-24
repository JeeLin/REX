import client from "@/api/client";

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  id?: string;
}

export interface AiConfigResponse {
  provider: string;
  model: string;
  base_url: string;
  configured: boolean;
}

export interface UpdateAiConfigRequest {
  provider?: string;
  api_key?: string;
  model?: string;
  base_url?: string;
}

export interface AiContext {
  database?: string;
  tables?: string[];
  dialect?: string;
}

export async function getAiConfig(): Promise<AiConfigResponse> {
  const response = await client.get("/ai/config");
  return response.data.data;
}

export async function updateAiConfig(
  data: UpdateAiConfigRequest,
): Promise<void> {
  await client.put("/ai/config", data);
}

export async function sendAiMessage(
  messages: ChatMessage[],
  context: AiContext,
): Promise<Response> {
  const response = await fetch('/api/ai/chat', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${localStorage.getItem('rex-token') || ''}`
    },
    body: JSON.stringify({ messages, context })
  })
  return response
}
