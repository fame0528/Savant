import { useState, useRef, useEffect } from "react";
import { useAppStore } from "@/lib/store";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function AiChat() {
  const { chatMessages, chatInput, chatStreaming, setChatInput, addChatMessage, setChatStreaming, chatOpen } = useAppStore();
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const [model, setModel] = useState("default");

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatMessages]);

  const handleSubmit = async () => {
    if (!chatInput.trim() || chatStreaming) return;

    const userContent = chatInput.trim();
    setChatInput("");
    addChatMessage({ role: "user", content: userContent });
    setChatStreaming(true);

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const response = await invoke<string>("gateway_send_chat", {
        message: userContent,
      });
      addChatMessage({ role: "assistant", content: response });
    } catch (e) {
      addChatMessage({
        role: "system",
        content: `Error: ${String(e)}`,
      });
    } finally {
      setChatStreaming(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  if (!chatOpen) return null;

  return (
    <div className="w-96 bg-void border-l border-primary/15 flex flex-col shrink-0">
      {/* Header */}
      <div className="p-3 border-b border-primary/15 flex items-center justify-between">
        <span className="text-primary font-bold text-sm">AI Chat</span>
        <select
          value={model}
          onChange={(e) => setModel(e.target.value)}
          className="bg-void/50 text-primary/60 text-xs border border-primary/15 rounded px-2 py-1"
        >
          <option value="default">Default</option>
          <option value="claude">Claude</option>
          <option value="gpt-4">GPT-4</option>
        </select>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {chatMessages.length === 0 && (
          <div className="text-primary/30 text-xs text-center mt-8">
            <p>Welcome to Savant CLI Companion</p>
            <p className="mt-2">Type a message to chat with your AI assistant.</p>
          </div>
        )}
        {chatMessages.map((msg) => (
          <div
            key={msg.id}
            className={`rounded-lg p-3 text-xs ${
              msg.role === "user"
                ? "bg-primary/10 border border-primary/20 ml-4"
                : msg.role === "system"
                ? "bg-warning/5 border border-warning/20 text-warning/70"
                : "bg-success/5 border border-success/15"
            }`}
          >
            <div className="text-primary/40 text-[10px] mb-1 uppercase">
              {msg.role}
            </div>
            <div className="prose prose-invert prose-xs max-w-none">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>
                {msg.content}
              </ReactMarkdown>
            </div>
          </div>
        ))}
        {chatStreaming && (
          <div className="text-accent text-xs animate-pulse">Thinking...</div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-3 border-t border-primary/15">
        <div className="flex gap-2">
          <textarea
            value={chatInput}
            onChange={(e) => setChatInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a message... (Enter to send, Shift+Enter for newline)"
            className="flex-1 bg-void/50 border border-primary/15 rounded-lg px-3 py-2 text-xs text-success placeholder-primary/20 resize-none focus:outline-none focus:border-primary/40 min-h-[60px] max-h-[120px]"
            rows={2}
          />
        </div>
        <div className="flex justify-between items-center mt-2">
          <span className="text-[10px] text-primary/20">
            {chatInput.length} chars
          </span>
          <button
            onClick={handleSubmit}
            disabled={!chatInput.trim() || chatStreaming}
            className="px-3 py-1 bg-primary/15 text-primary text-xs rounded hover:bg-primary/25 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
