import { useAppStore } from "@/lib/store";

export function AchievementsPanel() {
  const { game } = useAppStore();

  if (!game) {
    return (
      <div className="flex flex-1 items-center justify-center text-primary/30 text-sm">
        Loading...
      </div>
    );
  }

  const sorted = [...game.achievements].sort((a, b) => {
    if (a.unlocked_at && b.unlocked_at) return a.unlocked_at - b.unlocked_at;
    if (a.unlocked_at) return -1;
    if (b.unlocked_at) return 1;
    return a.name.localeCompare(b.name);
  });

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="p-4 border-b border-primary/15">
        <div className="text-xs text-success">{game.unlocked_achievements}/{game.total_achievements} unlocked</div>
      </div>
      <div className="p-2 space-y-1">
        {sorted.map((a) => {
          const unlocked = a.unlocked_at !== null;
          return (
            <div
              key={a.id}
              className={`flex items-start gap-3 px-3 py-2 rounded ${
                unlocked ? "bg-primary/8" : "bg-primary/3 opacity-50"
              }`}
            >
              <span className="text-lg mt-0.5">{unlocked ? a.icon : "\u{1f512}"}</span>
              <div className="flex-1 min-w-0">
                <div className={`text-xs font-bold ${unlocked ? "text-success" : "text-primary/40"}`}>
                  {a.name}
                </div>
                <div className="text-[10px] text-primary/40 leading-tight mt-0.5">
                  {a.description}
                </div>
                {unlocked && a.unlocked_at && (
                  <div className="text-[9px] text-primary/30 mt-0.5">
                    Unlocked {new Date(a.unlocked_at * 1000).toLocaleDateString()}
                  </div>
                )}
              </div>
              {unlocked && <span className="text-success text-xs">\u{2714}\u{fe0f}</span>}
            </div>
          );
        })}
      </div>
    </div>
  );
}
