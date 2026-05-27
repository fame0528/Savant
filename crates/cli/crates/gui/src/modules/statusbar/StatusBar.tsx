import { useAppStore } from "@/lib/store";

export function StatusBar() {
  const { gatewayConnected, activePtyId, ptySessions, chatStreaming, game, setActivePanel } = useAppStore();
  const sessionCount = Object.keys(ptySessions).length;

  return (
    <div className="h-6 bg-void border-t border-primary/15 flex items-center px-3 text-xs text-primary/50 shrink-0 gap-4">
      <span className={gatewayConnected ? "text-success/70" : "text-warning/70"}>
        {gatewayConnected ? "\u25cf Connected" : "\u25cb Offline"}
      </span>
      <span>Terminals: {sessionCount}</span>
      <span>Active: {activePtyId?.slice(0, 8) ?? "none"}</span>
      {chatStreaming && (
        <span className="text-accent animate-pulse">AI streaming...</span>
      )}
      {game && !game.pet.is_dead && (
        <button
          onClick={() => setActivePanel("pet")}
          className="flex items-center gap-1 hover:text-success transition-colors"
          title={`${game.pet.stage} \u2022 Streak: ${game.current_streak}d`}
        >
          <span>{game.pet.stage_icon}</span>
          <span className="text-primary/40">
            {game.current_streak > 0 && `\u{1f525}${game.current_streak}`}
          </span>
          {game.pet.is_sick && <span className="text-warning">\u{1f912}</span>}
          {game.pet.poop_count > 0 && game.pet.poop_count >= 3 && <span className="text-warning">\u{1f4a9}</span>}
        </button>
      )}
      {game?.pet.is_dead && (
        <button onClick={() => setActivePanel("pet")} className="text-warning hover:text-success transition-colors" title="Pet has passed away">
          \u{1f480} Restart
        </button>
      )}
      <span className="ml-auto text-primary/30">
        j/k scroll \u00b7 Tab panels \u00b7 Ctrl+P commands
      </span>
    </div>
  );
}
