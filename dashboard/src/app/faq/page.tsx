"use client";

export default function FAQPage() {
  return (
    <div style={{ padding: "40px", maxWidth: "800px", margin: "0 auto" }}>
      <h1 style={{ color: "var(--accent)", fontSize: "24px", marginBottom: "32px", letterSpacing: "2px" }}>FAQ</h1>

      <Section title="GETTING STARTED">
        <Q q="How do I start Savant?">
          Run <code style={codeStyle}>npm run dev</code> from the project root. This starts the desktop app with hot reload for both the dashboard and Rust backend.
        </Q>
        <Q q="What does npm run dev do?">
          1. Starts the Next.js dashboard dev server (port 3000)
          2. Compiles the Rust backend (debug mode, cached)
          3. Opens the Savant desktop window
          4. Dashboard connects to the gateway on port 8080
        </Q>
        <Q q="How do I access the debug console?">
          Click the <strong>SWARM_NOMINAL</strong> status button in the top right. It shows all Rust tracing logs with color-coded levels, copy, expand, and pause on highlight.
        </Q>
      </Section>

      <Section title="OLLAMA SETUP">
        <Q q="What models do I need?">
          <ul style={{ margin: 0, paddingLeft: "20px" }}>
            <li><strong>Chat:</strong> Any model you prefer (e.g., llama3, qwen2.5, deepseek-coder)</li>
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
          Savant falls back to <strong>fastembed</strong> for embeddings (lower quality, but no Ollama dependency). Vision requires Ollama to be running. Chat uses OpenRouter regardless.
        </Q>
      </Section>

      <Section title="OPENROUTER">
        <Q q="How does the key system work?">
          Savant uses a <strong>master key</strong> system. Set <code style={codeStyle}>OR_MASTER_KEY</code> in your root <code style={codeStyle}>.env</code> file. At runtime, derivative keys are created for each agent session and never persisted. This keeps your keys secure.
        </Q>
        <Q q="Where do I set my API key?">
          In <code style={codeStyle}>C:\Users\spenc\dev\Savant\.env</code> (project root), add:
          <pre style={preStyle}>OR_MASTER_KEY=sk-or-v1-your-key-here</pre>
        </Q>
      </Section>

      <Section title="MODELS">
        <Q q="What is the chat model?">
          The LLM that powers conversations. Default: <code style={codeStyle}>stepfun/step-3.5-flash:free</code> (free tier via OpenRouter). You can use any model available on OpenRouter.
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
          1. Clear old vector data: <code style={codeStyle}>rm -r data/memory</code>
          2. Restart the app to reinitialize with correct dimensions
          3. Verify Ollama embedding model is pulled
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
