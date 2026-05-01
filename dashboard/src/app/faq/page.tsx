"use client";

export default function FAQPage() {
  return (
    <div style={{ padding: "40px", width: "100%", maxWidth: "1200px", margin: "0 auto" }}>
      <h1 style={{ color: "var(--accent)", fontSize: "24px", marginBottom: "32px", letterSpacing: "2px" }}>FAQ</h1>

      <Section title="GETTING STARTED">
        <Q q="How do I start Savant?">
          Launch the Savant desktop application. The backend gateway starts automatically and the dashboard connects to it on port 8080.
        </Q>
        <Q q="How do I access the debug console?">
          Click the <strong>SWARM_NOMINAL</strong> status button in the top right header. It shows all Rust tracing logs with color-coded levels, copy, expand, and pause on highlight.
        </Q>
        <Q q="What models does Savant use?">
          Savant uses OpenRouter for chat (default: <code style={codeStyle}>qwen/qwen3.6-plus:free</code>), Ollama for embeddings (<code style={codeStyle}>qwen3-embedding:4b</code>), and Ollama for vision (<code style={codeStyle}>qwen3-vl</code>).
        </Q>
      </Section>

      <Section title="OLLAMA SETUP">
        <Q q="What models do I need?">
          <ul style={{ margin: 0, paddingLeft: "20px" }}>
            <li><strong>Chat:</strong> Any model on OpenRouter (default: qwen/qwen3.6-plus:free)</li>
            <li><strong>Embedding:</strong> <code style={codeStyle}>qwen3-embedding:4b</code> (2560 dims)</li>
            <li><strong>Vision:</strong> <code style={codeStyle}>qwen3-vl</code></li>
          </ul>
        </Q>
        <Q q="How do I pull the models?">
          <pre style={preStyle}>
{`ollama pull qwen3-embedding:4b
ollama pull qwen3-vl`}
          </pre>
        </Q>
        <Q q="What if Ollama isn't running?">
          Savant auto-starts Ollama if it detects the installation. If Ollama is unavailable, embedding falls back to fastembed (lower quality). Vision requires Ollama.
        </Q>
      </Section>

      <Section title="OPENROUTER">
        <Q q="How does the key system work?">
          Savant uses a <strong>master key</strong> system. Set <code style={codeStyle}>OR_MASTER_KEY</code> in your project root <code style={codeStyle}>.env</code> file. At runtime, derivative keys are created for each agent session and never persisted. This keeps your keys secure.
        </Q>
        <Q q="Where do I set my API key?">
          In your project root <code style={codeStyle}>.env</code> file, add:
          <pre style={preStyle}>OR_MASTER_KEY=sk-or-v1-your-key-here</pre>
        </Q>
      </Section>

      <Section title="FEATURES">
        <Q q="What is the Continuous Awareness Architecture?">
          Savant v0.2.0 introduces background processing capabilities including a dream engine (NREM/REM memory consolidation), global workspace for signal broadcasting, semantic context windows, temporal memory decay, and continuous agent safety with taint tracing and circuit breakers.
        </Q>
        <Q q="What is the chat model?">
          The LLM that powers conversations. Default: <code style={codeStyle}>qwen/qwen3.6-plus:free</code> (free tier via OpenRouter). You can use any model available on OpenRouter.
        </Q>
        <Q q="What is the embedding model?">
          Converts text to vectors for semantic memory search. Default: <code style={codeStyle}>qwen3-embedding:4b</code> (via Ollama, 2560 dims).
        </Q>
        <Q q="What is the vision model?">
          Describes images. Default: <code style={codeStyle}>qwen3-vl</code> (via Ollama). Used when images are attached to messages.
        </Q>
      </Section>

      <Section title="TROUBLESHOOTING">
        <Q q="Chat not responding">
          1. Check the debug console for errors
          2. Verify the model in Settings exists on OpenRouter
          3. Check that OR_MASTER_KEY is set in .env
          4. Look for &quot;OpenRouter API error&quot; in logs
        </Q>
        <Q q="Images not loading">
          1. Gateway must be running (check SWARM_NOMINAL status)
          2. Avatar should be at <code style={codeStyle}>workspaces/workspace-savant/avatar.png</code>
        </Q>
        <Q q="Memory search errors">
          1. Clear old vector data: delete <code style={codeStyle}>data/memory</code> directory
          2. Restart the app to reinitialize with correct dimensions
          3. Verify Ollama embedding model is pulled
        </Q>
        <Q q="Health page shows 0 agents">
          The health page reads from the shared WebSocket connection. Ensure the gateway is running and the WebSocket shows SWARM_NOMINAL in the header.
        </Q>
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section style={{ marginBottom: "40px" }}>
      <h2 style={{ color: "var(--accent)", fontSize: "14px", letterSpacing: "2px", marginBottom: "16px", borderLeft: "3px solid var(--accent)", paddingLeft: "12px" }}>{title}</h2>
      {children}
    </section>
  );
}

function Q({ q, children }: { q: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: "20px" }}>
      <div style={{ fontSize: "13px", fontWeight: 900, color: "#fff", marginBottom: "6px" }}>{q}</div>
      <div style={{ fontSize: "13px", lineHeight: "1.6", opacity: 0.7, paddingLeft: "12px" }}>{children}</div>
    </div>
  );
}

const codeStyle: React.CSSProperties = {
  background: "rgba(0, 213, 255, 0.1)",
  padding: "2px 6px",
  borderRadius: "4px",
  fontSize: "12px",
  color: "var(--accent)",
};

const preStyle: React.CSSProperties = {
  background: "rgba(0,0,0,0.3)",
  padding: "12px",
  borderRadius: "8px",
  fontSize: "12px",
  fontFamily: "monospace",
  color: "var(--accent)",
  margin: "8px 0",
  overflow: "auto",
};
