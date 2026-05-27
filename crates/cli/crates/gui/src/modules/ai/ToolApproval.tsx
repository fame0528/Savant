import { useState } from "react";

interface ToolApprovalRequest {
  id: string;
  toolName: string;
  input: Record<string, unknown>;
  timestamp: number;
}

interface ToolApprovalProps {
  request: ToolApprovalRequest;
  onApprove: (id: string) => void;
  onDeny: (id: string) => void;
}

export function ToolApproval({ request, onApprove, onDeny }: ToolApprovalProps) {
  const input = request.input;

  const renderPreview = () => {
    if (request.toolName === "bash_run" || request.toolName === "bash_background") {
      const cwd = typeof input.cwd === "string" ? input.cwd : null;
      return (
        <div className="space-y-1.5">
          {cwd && (
            <div className="font-mono text-[10.5px] text-primary/50">{cwd}</div>
          )}
          <pre className="max-h-40 overflow-auto rounded-md bg-primary/5 p-2 font-mono text-[11px] leading-relaxed text-success/70">
            {String(input.command ?? "")}
          </pre>
        </div>
      );
    }
    if (request.toolName === "write_file") {
      const content = typeof input.content === "string" ? input.content : "";
      const lines = content ? content.split("\n").length : 0;
      return (
        <div className="space-y-0.5 font-mono text-[11px]">
          <div className="text-primary/50">{String(input.path ?? "")}</div>
          <div className="text-[10.5px] text-primary/30">
            {lines} line{lines === 1 ? "" : "s"} · review in the diff tab
          </div>
        </div>
      );
    }
    if (request.toolName === "edit") {
      const oldStr = typeof input.old_string === "string" ? input.old_string : "";
      const newStr = typeof input.new_string === "string" ? input.new_string : "";
      const removed = oldStr ? oldStr.split("\n").length : 0;
      const added = newStr ? newStr.split("\n").length : 0;
      return (
        <div className="space-y-0.5 font-mono text-[11px]">
          <div className="text-primary/50">
            {String(input.path ?? "")}
            {input.replace_all ? " · replace all" : ""}
          </div>
          <div className="text-[10.5px] text-primary/30">
            −{removed} / +{added} line{added === 1 && removed === 1 ? "" : "s"} · review in the diff tab
          </div>
        </div>
      );
    }
    return (
      <pre className="overflow-auto rounded-md bg-primary/5 p-2 font-mono text-[11px] leading-relaxed text-success/70">
        {JSON.stringify(input, null, 2)}
      </pre>
    );
  };

  const TOOL_META: Record<string, { label: string; color: string }> = {
    write_file: { label: "Write file", color: "text-success" },
    edit: { label: "Edit file", color: "text-warning" },
    multi_edit: { label: "Edit file (batch)", color: "text-warning" },
    create_directory: { label: "Create directory", color: "text-primary" },
    bash_run: { label: "Run shell command", color: "text-accent" },
    bash_background: { label: "Spawn background process", color: "text-accent" },
  };

  const meta = TOOL_META[request.toolName] ?? { label: request.toolName, color: "text-success" };

  return (
    <div className="rounded-lg border border-primary/20 bg-void shadow-lg animate-slide-in">
      <div className="flex items-center gap-2 border-b border-primary/10 px-3 py-2">
        <span className="size-1.5 shrink-0 rounded-full bg-warning animate-pulse" />
        <span className={`text-xs font-medium ${meta.color}`}>{meta.label}</span>
        <span className="ml-auto text-[10px] text-primary/30">needs approval</span>
      </div>
      <div className="px-3 py-2.5">{renderPreview()}</div>
      <div className="flex items-center justify-end gap-1.5 border-t border-primary/10 px-3 py-2">
        <button
          onClick={() => onDeny(request.id)}
          className="h-7 gap-1.5 px-3 text-xs text-alert/70 hover:text-alert hover:bg-alert/10 rounded transition-colors"
        >
          Deny
        </button>
        <button
          onClick={() => onApprove(request.id)}
          className="h-7 gap-1.5 px-3 text-xs bg-success/10 text-success rounded hover:bg-success/20 transition-colors"
        >
          Approve
        </button>
      </div>
    </div>
  );
}

export function ToolApprovalOverlay() {
  const [pendingRequests, setPendingRequests] = useState<ToolApprovalRequest[]>([]);

  const handleApprove = async (id: string) => {
    setPendingRequests((prev) => prev.filter((r) => r.id !== id));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("gateway_send_chat", {
        message: JSON.stringify({ type: "ToolApproval", toolCallId: id, approved: true }),
      });
    } catch (e) {
      console.error("Failed to send approval:", e);
    }
  };

  const handleDeny = async (id: string) => {
    setPendingRequests((prev) => prev.filter((r) => r.id !== id));
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("gateway_send_chat", {
        message: JSON.stringify({ type: "ToolDenial", toolCallId: id, reason: "User denied" }),
      });
    } catch (e) {
      console.error("Failed to send denial:", e);
    }
  };

  if (pendingRequests.length === 0) return null;

  return (
    <div className="absolute inset-0 z-10 flex flex-col gap-2 bg-void/85 backdrop-blur-sm p-4 overflow-y-auto">
      {pendingRequests.map((req) => (
        <ToolApproval
          key={req.id}
          request={req}
          onApprove={handleApprove}
          onDeny={handleDeny}
        />
      ))}
    </div>
  );
}
