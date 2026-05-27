import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/lib/store";

export function PetDisplay() {
  const { game, gameEvents, addGameEvent, setGamificationState } = useAppStore();

  const refresh = useCallback(async () => {
    try {
      const state = await invoke<any>("gamification_get_state");
      setGamificationState(state);
    } catch { /* ignore */ }
  }, [setGamificationState]);

  const act = useCallback(async (cmd: string, args?: Record<string, unknown>) => {
    try {
      const events = await invoke<string[]>(cmd, args ?? {});
      for (const e of events) addGameEvent(e);
      await refresh();
    } catch (e) {
      addGameEvent(`Error: ${e}`);
    }
  }, [addGameEvent, refresh]);

  if (!game) {
    return (
      <div className="flex flex-1 items-center justify-center text-primary/30 text-sm">
        Loading pet state...
      </div>
    );
  }

  const p = game.pet;
  const hungerPct = Math.round(p.hunger * 100);
  const happyPct = Math.round(p.happiness * 100);
  const healthPct = Math.round(p.health * 100);
  const cleanPct = Math.round(p.cleanliness * 100);
  const discPct = Math.round(p.discipline * 100);

  const bar = (val: number, color: string) => (
    <div className="h-1.5 bg-primary/10 rounded-full overflow-hidden flex-1">
      <div
        className={`h-full rounded-full transition-all duration-500 ${color}`}
        style={{ width: `${Math.max(0, Math.min(100, val))}%` }}
      />
    </div>
  );

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      {/* Pet Avatar */}
      <div className="flex flex-col items-center py-6 border-b border-primary/15">
        <div className={`text-6xl mb-2 transition-all duration-300 ${p.is_dead ? "grayscale" : p.is_sleeping ? "opacity-50" : ""}`}>
          {p.is_dead ? "\u{1f480}" : p.stage_icon}
        </div>
        <div className="flex items-center gap-2 text-sm">
          <span className="text-primary font-bold">{p.stage}</span>
          <span title={p.mood}>{p.mood_icon}</span>
        </div>
        <div className="text-primary/40 text-xs">
          {Math.floor(p.age_hours)}h old
          {p.is_sleeping && " \u{1f4a4} Sleeping"}
          {p.is_sick && " \u{1f912} Sick"}
          {p.is_dead && " \u{2620} Deceased"}
        </div>
      </div>

      {/* Stats */}
      <div className="px-4 py-3 space-y-2 text-xs border-b border-primary/15">
        <StatRow label="\u{1f356} Hunger" value={hungerPct} bar={bar(hungerPct, "bg-orange-500")} />
        <StatRow label="\u{1f60a} Happiness" value={happyPct} bar={bar(happyPct, "bg-yellow-500")} />
        <StatRow label="\u{2764}\u{fe0f} Health" value={healthPct} bar={bar(healthPct, "bg-red-500")} />
        <StatRow label="\u{1f4a7} Cleanliness" value={cleanPct} bar={bar(cleanPct, "bg-cyan-500")} />
        <StatRow label="\u{1f4aa} Discipline" value={discPct} bar={bar(discPct, "bg-purple-500")} />
        <div className="flex justify-between text-primary/50 pt-1">
          <span>Weight: {p.weight.toFixed(1)}x</span>
          {p.poop_count > 0 && <span className="text-warning">{p.poop_count}x {'\u{1f4a9}'}</span>}
          <span>Mistakes: {p.care_mistakes}</span>
        </div>
        <div className="flex justify-between text-primary/50">
          <span>XP: {game.xp}</span>
          <span>Streak: {game.current_streak}{'\u{1f525}'}</span>
          <span>Achievements: {game.unlocked_achievements}/{game.total_achievements}</span>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="p-4 grid grid-cols-2 gap-2 border-b border-primary/15">
        <ActionBtn icon="\u{1f372}" label="Feed Meal" onClick={() => act("gamification_feed", { feedType: "meal" })} disabled={p.is_dead} />
        <ActionBtn icon="\u{1f36a}" label="Feed Snack" onClick={() => act("gamification_feed", { feedType: "snack" })} disabled={p.is_dead} />
        <ActionBtn icon="\u{1f3ae}" label="Play" onClick={() => act("gamification_play")} disabled={p.is_dead} />
        <ActionBtn icon="\u{1f9f4}" label="Clean" onClick={() => act("gamification_clean")} disabled={p.is_dead || p.poop_count === 0} />
        <ActionBtn icon="\u{1f48a}" label="Medicine" onClick={() => act("gamification_medicine")} disabled={p.is_dead || !p.is_sick} />
        <ActionBtn icon="\u{1f6ab}" label="Discipline" onClick={() => act("gamification_discipline")} disabled={p.is_dead} />
        <ActionBtn icon={p.is_sleeping ? "\u{2600}\u{fe0f}" : "\u{1f634}"} label={p.is_sleeping ? "Wake Up" : "Sleep"} onClick={() => act("gamification_sleep")} disabled={p.is_dead} />
        <ActionBtn icon="\u{270b}" label="Pet" onClick={() => act("gamification_pet")} disabled={p.is_dead} />
      </div>

      {/* Check-in & Reset */}
      <div className="p-4 flex gap-2 border-b border-primary/15">
        <button
          onClick={() => act("gamification_checkin")}
          className="flex-1 px-3 py-2 rounded bg-primary/10 hover:bg-primary/20 text-success text-xs transition-colors"
        >
          \u{2705} Daily Check-in
        </button>
        {p.is_dead && (
          <button
            onClick={() => act("gamification_reset_egg")}
            className="flex-1 px-3 py-2 rounded bg-warning/20 hover:bg-warning/30 text-warning text-xs transition-colors"
          >
            {'\u{1f95a}'} New Egg
          </button>
        )}
      </div>

      {/* Event Log */}
      {gameEvents.length > 0 && (
        <div className="p-4 overflow-y-auto max-h-40">
          <div className="text-primary/30 text-xs mb-1">Events</div>
          <div className="space-y-0.5">
            {gameEvents.map((e, i) => (
              <div key={i} className="text-[10px] text-primary/50 leading-tight">{e}</div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function StatRow({ label, value, bar }: { label: string; value: number; bar: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-primary/60 w-24 shrink-0">{label}</span>
      {bar}
      <span className="text-primary/60 w-8 text-right tabular-nums">{value}%</span>
    </div>
  );
}

function ActionBtn({ icon, label, onClick, disabled }: { icon: string; label: string; onClick: () => void; disabled: boolean }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`flex items-center gap-2 px-3 py-2 rounded text-xs transition-colors ${
        disabled
          ? "bg-primary/5 text-primary/20 cursor-not-allowed"
          : "bg-primary/10 hover:bg-primary/20 text-success hover:text-primary"
      }`}
    >
      <span>{icon}</span>
      <span>{label}</span>
    </button>
  );
}
