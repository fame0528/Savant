import { useState, useEffect } from "react";

export function PreviewPane() {
  const [url, setUrl] = useState<string>("");
  const [detectedUrls, setDetectedUrls] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  // Auto-detect common dev server ports
  useEffect(() => {
    const commonPorts = [3000, 5173, 8080, 8000, 4200, 5000];
    const checkUrls = async () => {
      const found: string[] = [];
      for (const port of commonPorts) {
        try {
          const testUrl = `http://localhost:${port}`;
          const ctrl = new AbortController();
          const timeout = setTimeout(() => ctrl.abort(), 1000);
          await fetch(testUrl, { signal: ctrl.signal, mode: "no-cors" });
          clearTimeout(timeout);
          found.push(testUrl);
        } catch {
          // Port not available
        }
      }
      setDetectedUrls(found);
    };
    checkUrls();
    const interval = setInterval(checkUrls, 10000);
    return () => clearInterval(interval);
  }, []);

  const handleNavigate = (navUrl: string) => {
    setUrl(navUrl);
    setLoading(true);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-primary/10">
        <span className="text-primary font-bold text-xs">Preview</span>
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="http://localhost:3000"
          className="flex-1 bg-void/50 border border-primary/15 rounded px-2 py-1 text-[10px] text-success placeholder-primary/20 focus:outline-none focus:border-primary/30"
          onKeyDown={(e) => { if (e.key === "Enter") handleNavigate(url); }}
        />
        <button
          onClick={() => handleNavigate(url)}
          className="text-[10px] text-primary/40 hover:text-primary transition-colors"
        >
          Go
        </button>
      </div>

      {/* Detected URLs */}
      {detectedUrls.length > 0 && !url && (
        <div className="px-3 py-2 border-b border-primary/5">
          <div className="text-[10px] text-primary/30 mb-1">Detected dev servers:</div>
          {detectedUrls.map((u) => (
            <button
              key={u}
              onClick={() => handleNavigate(u)}
              className="block text-xs text-primary/60 hover:text-primary transition-colors"
            >
              {u}
            </button>
          ))}
        </div>
      )}

      {/* Preview content */}
      <div className="flex-1 bg-void/50 flex items-center justify-center">
        {loading ? (
          <div className="text-primary/30 text-xs animate-pulse">Loading...</div>
        ) : url ? (
          <iframe
            src={url}
            className="w-full h-full border-0"
            title="Web Preview"
            onLoad={() => setLoading(false)}
            sandbox="allow-scripts allow-same-origin allow-forms"
          />
        ) : (
          <div className="text-primary/20 text-xs text-center">
            <p className="text-2xl mb-2">🌐</p>
            <p>Enter a URL or auto-detect a local dev server</p>
          </div>
        )}
      </div>
    </div>
  );
}
