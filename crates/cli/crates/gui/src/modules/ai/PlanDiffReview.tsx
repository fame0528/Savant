import { useState } from "react";

interface QueuedEdit {
  id: string;
  kind: "edit" | "multi_edit" | "write_file" | "create_directory";
  path: string;
  originalContent: string;
  proposedContent: string;
  isNewFile: boolean;
  description?: string;
}

interface PlanDiffReviewProps {
  queue: QueuedEdit[];
  onApplyAll: () => void;
  onRejectAll: () => void;
  onRejectOne: (id: string) => void;
}

function diffStats(original: string, proposed: string): { added: number; removed: number } {
  const a = original.split("\n");
  const b = proposed.split("\n");
  const setA = new Set(a);
  const setB = new Set(b);
  let added = 0;
  let removed = 0;
  for (const line of b) if (!setA.has(line)) added++;
  for (const line of a) if (!setB.has(line)) removed++;
  return { added, removed };
}

function basename(p: string): string {
  const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return i >= 0 ? p.slice(i + 1) : p;
}

export function PlanDiffReview({ queue, onApplyAll, onRejectAll, onRejectOne }: PlanDiffReviewProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);

  if (queue.length === 0) return null;

  return (
    <div className="absolute inset-0 z-10 flex flex-col bg-void/90 backdrop-blur-xl">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-primary/15 px-4 py-2">
        <div className="flex flex-col">
          <span className="text-sm font-bold text-primary">Plan Review</span>
          <span className="text-[10.5px] text-primary/40">
            {queue.length} pending change{queue.length === 1 ? "" : "s"}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          <button
            onClick={onRejectAll}
            className="h-7 gap-1.5 px-3 text-[11px] text-alert/70 hover:text-alert hover:bg-alert/10 rounded transition-colors"
          >
            Discard all
          </button>
          <button
            onClick={onApplyAll}
            className="h-7 gap-1.5 px-3 text-[11px] bg-success/10 text-success rounded hover:bg-success/20 transition-colors"
          >
            Apply {queue.length}
          </button>
        </div>
      </div>

      {/* Edit list */}
      <ul className="flex-1 overflow-y-auto p-3 space-y-1.5">
        {queue.map((q) => (
          <PlanRow
            key={q.id}
            item={q}
            expanded={expandedId === q.id}
            onToggle={() => setExpandedId(expandedId === q.id ? null : q.id)}
            onReject={() => onRejectOne(q.id)}
          />
        ))}
      </ul>
    </div>
  );
}

function PlanRow({
  item,
  expanded,
  onToggle,
  onReject,
}: {
  item: QueuedEdit;
  expanded: boolean;
  onToggle: () => void;
  onReject: () => void;
}) {
  const isDir = item.kind === "create_directory";
  const isNew = item.isNewFile && !isDir;
  const stats = isDir ? null : diffStats(item.originalContent, item.proposedContent);

  return (
    <li className="overflow-hidden rounded-md border border-primary/10 bg-void">
      <div className="flex items-start gap-2 px-2.5 py-1.5">
        <button
          type="button"
          onClick={() => !isDir && onToggle()}
          disabled={isDir}
          className={`mt-0.5 shrink-0 text-primary/40 transition-transform ${expanded ? "rotate-180" : ""} ${isDir ? "invisible" : ""}`}
        >
          ▼
        </button>
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-1.5 font-mono text-[11.5px]">
            <span className="truncate text-success/80">{basename(item.path)}</span>
            {isNew && (
              <span className="text-[10px] text-success">new</span>
            )}
          </div>
          <div className="truncate font-mono text-[10px] text-primary/40">{item.path}</div>
          {stats ? (
            <div className="mt-0.5 flex items-center gap-2 text-[10px] tabular-nums">
              <span className="text-success">+{stats.added}</span>
              <span className="text-alert">−{stats.removed}</span>
              <span className="text-primary/30">{item.kind}</span>
            </div>
          ) : (
            <div className="mt-0.5 text-[10px] text-primary/30">
              {item.description ?? "create directory"}
            </div>
          )}
        </div>
        <button
          onClick={onReject}
          className="size-5 shrink-0 text-primary/20 hover:text-alert opacity-0 group-hover:opacity-100 transition-opacity rounded"
        >
          ×
        </button>
      </div>
      {expanded && !isDir && (
        <div className="border-t border-primary/10 bg-primary/5 px-2.5 py-2">
          <UnifiedDiffPreview original={item.originalContent} proposed={item.proposedContent} />
        </div>
      )}
    </li>
  );
}

function UnifiedDiffPreview({ original, proposed }: { original: string; proposed: string }) {
  const a = original.split("\n");
  const b = proposed.split("\n");
  const setA = new Set(a);
  const setB = new Set(b);

  const lines: Array<{ kind: "add" | "del" | "ctx"; text: string }> = [];
  for (const l of a) if (!setB.has(l)) lines.push({ kind: "del", text: l });
  for (const l of b) if (!setA.has(l)) lines.push({ kind: "add", text: l });

  if (lines.length === 0) {
    return <div className="text-[11px] italic text-primary/30">no line-level changes</div>;
  }

  const MAX = 80;
  const shown = lines.slice(0, MAX);
  const rest = lines.length - shown.length;

  return (
    <div className="overflow-hidden rounded border border-primary/10 font-mono text-[11px] leading-relaxed">
      <div className="max-h-72 overflow-auto">
        {shown.map((l, i) => (
          <div
            key={i}
            className={`flex whitespace-pre ${
              l.kind === "add"
                ? "bg-success/10 text-success"
                : "bg-alert/10 text-alert/70"
            }`}
          >
            <span className="w-4 shrink-0 select-none px-1 text-center opacity-70">
              {l.kind === "add" ? "+" : "−"}
            </span>
            <span className="min-w-0 flex-1 overflow-x-auto pr-2">{l.text || " "}</span>
          </div>
        ))}
        {rest > 0 && (
          <div className="px-2 py-1 text-[10px] italic text-primary/30">
            … {rest} more changes
          </div>
        )}
      </div>
    </div>
  );
}
