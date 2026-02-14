import { retrieveApiKey } from "./commands";

const BASE_URL =
  "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:streamGenerateContent";

export class GeminiError extends Error {
  constructor(
    message: string,
    public status?: number,
    public retryable: boolean = false
  ) {
    super(message);
    this.name = "GeminiError";
  }
}

export async function streamGemini(
  prompt: string,
  onChunk: (text: string) => void,
  signal?: AbortSignal,
  systemPrompt?: string
): Promise<void> {
  const apiKey = await retrieveApiKey();
  if (!apiKey) {
    throw new GeminiError("No API key configured. Add it in Settings.");
  }

  const body: Record<string, unknown> = {
    contents: [{ parts: [{ text: prompt }] }],
  };

  if (systemPrompt) {
    body.systemInstruction = { parts: [{ text: systemPrompt }] };
  }

  console.log(`[Gemini] Requesting: ${BASE_URL}`);
  console.log(`[Gemini] Model: gemini-3-flash-preview`);

  const res = await fetch(`${BASE_URL}?key=${apiKey}&alt=sse`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal,
  });

  if (!res.ok) {
    const errorText = await res.text();
    console.error(`[Gemini] API Error ${res.status}:`, errorText);

    if (res.status === 429) {
      throw new GeminiError("Rate limited. Please wait.", 429, true);
    }
    if (res.status === 401 || res.status === 403) {
      throw new GeminiError(
        "Invalid API key. Update it in Settings.",
        res.status
      );
    }
    throw new GeminiError(`Gemini API error ${res.status}: ${errorText}`);
  }

  const reader = res.body?.getReader();
  if (!reader) throw new GeminiError("No response body");

  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() || "";

    for (const line of lines) {
      if (line.startsWith("data: ")) {
        try {
          const json = JSON.parse(line.slice(6));
          const text = json?.candidates?.[0]?.content?.parts?.[0]?.text;
          if (text) onChunk(text);
        } catch {
          // Skip malformed SSE lines
        }
      }
    }
  }
}
