import { useAppStore, type ActivePanel } from "@/lib/store";

const NAV_ITEMS: { id: ActivePanel; label: string; icon: string }[] = [
  { id: "files", label: "Explorer", icon: "\u{1f4c1}" },
  { id: "terminal", label: "Terminal", icon: "\u{2b1b}" },
  { id: "search", label: "Search", icon: "\u{1f50d}" },
  { id: "git", label: "Git", icon: "\u{1f33f}" },
  { id: "pet", label: "Pet", icon: "\u{1f331}" },
];

export function Sidebar() {
  const { activePanel, setActivePanel, sidebarOpen } = useAppStore();

  if (!sidebarOpen) return null;

  return (
    <div className="w-48 bg-void border-r border-primary/15 flex flex-col">
      <div className="p-3 border-b border-primary/15">
        <span className="text-primary font-bold text-sm">SAVANT</span>
        <span className="text-primary/40 text-xs ml-1">v0.1.0</span>
      </div>
      <nav className="flex-1 p-2">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            onClick={() => setActivePanel(item.id)}
            className={`w-full text-left px-3 py-2 rounded text-sm transition-colors ${
              activePanel === item.id
                ? "bg-primary/15 text-primary"
                : "text-success/70 hover:bg-primary/5 hover:text-success"
            }`}
          >
            <span className="mr-2">{item.icon}</span>
            {item.label}
          </button>
        ))}
      </nav>
      <div className="p-3 border-t border-primary/15">
        <div className="text-xs text-primary/40">
          Sessions
        </div>
      </div>
    </div>
  );
}
