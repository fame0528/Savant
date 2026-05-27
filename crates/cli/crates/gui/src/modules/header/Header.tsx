import { useAppStore } from "@/lib/store";

export function Header({ onSettingsClick }: { onSettingsClick?: () => void }) {
  const { gatewayConnected, gatewayUrl, platform } = useAppStore();

  return (
    <div className="h-9 bg-void border-b border-primary/15 flex items-center px-3 shrink-0">
      <div className="flex items-center gap-3 text-xs">
        <span className="text-primary font-bold">SAVANT CLI COMPANION</span>
        <span className="text-primary/20">|</span>
        <span className="text-primary/40">{platform}</span>
        <span className="text-primary/20">|</span>
        <span className={gatewayConnected ? "text-success" : "text-warning"}>
          {gatewayConnected ? `● ${gatewayUrl}` : "○ Disconnected"}
        </span>
      </div>
      <div className="ml-auto flex items-center gap-2 text-xs text-primary/40">
        <button
          onClick={onSettingsClick}
          className="hover:text-primary transition-colors px-2 py-1 rounded hover:bg-primary/10"
        >
          Settings
        </button>
      </div>
    </div>
  );
}
