import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";
import { insertText, insertTextViaPaste } from "../lib/commands";
import { streamGemini } from "../lib/gemini";
import type { TextFieldBounds } from "../lib/commands";

export function OverlayView() {
  const [prompt, setPrompt] = useState("");
  const [response, setResponse] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    // Listen for trigger from Rust keystroke monitor
    const setupListener = async () => {
      const unlisten = await listen<TextFieldBounds>(
        "trigger-detected",
        async (event) => {
          const bounds = event.payload;
          const appWindow = getCurrentWindow();

          // Position overlay near text field
          const padding = 8;
          const y = bounds.y + bounds.height + padding;

          await appWindow.setPosition(
            new PhysicalPosition(Math.round(bounds.x), Math.round(y))
          );
          await appWindow.show();
          await appWindow.setFocus();

          // Reset state
          setPrompt("");
          setResponse("");
          setIsGenerating(false);
          inputRef.current?.focus();
        }
      );

      return unlisten;
    };

    const unlistenPromise = setupListener();

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const handleSubmit = async () => {
    if (!prompt.trim() || isGenerating) return;
    setIsGenerating(true);
    setResponse("");

    abortRef.current = new AbortController();

    try {
      await streamGemini(
        prompt,
        (chunk) => setResponse((prev) => prev + chunk),
        abortRef.current.signal
      );
    } catch (err: unknown) {
      const error = err as Error;
      if (error.name !== "AbortError") {
        setResponse(`Error: ${error.message}`);
      }
    } finally {
      setIsGenerating(false);
    }
  };

  const handleInsert = async () => {
    try {
      await insertText(response);
    } catch {
      // Fallback to paste method
      await insertTextViaPaste(response);
    }
    await getCurrentWindow().hide();
  };

  const handleCancel = async () => {
    abortRef.current?.abort();
    await getCurrentWindow().hide();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
    if (e.key === "Escape") {
      handleCancel();
    }
  };

  return (
    <div className="overlay-container">
      <textarea
        ref={inputRef}
        className="overlay-input"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Ask AI anything..."
        rows={2}
      />

      {response && <div className="overlay-response">{response}</div>}

      <div className="overlay-actions">
        <button onClick={handleCancel} className="btn-cancel">
          Cancel
        </button>
        <div className="btn-group">
          {response && (
            <button
              onClick={() => navigator.clipboard.writeText(response)}
              className="btn-secondary"
            >
              Copy
            </button>
          )}
          <button
            onClick={handleInsert}
            disabled={!response || isGenerating}
            className="btn-primary"
          >
            {isGenerating ? "Generating..." : "Insert"}
          </button>
        </div>
      </div>
    </div>
  );
}
