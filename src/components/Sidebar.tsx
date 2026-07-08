// Боковая навигация приложения.

import { clsx } from "clsx";
import { Download, Search, Settings } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { useAppStore, type View } from "@/store";

const NAV: { view: View; label: string; icon: LucideIcon }[] = [
  { view: "search", label: "Поиск", icon: Search },
  { view: "downloads", label: "Загрузки", icon: Download },
  { view: "settings", label: "Настройки", icon: Settings },
];

export function Sidebar() {
  const view = useAppStore((s) => s.view);
  const setView = useAppStore((s) => s.setView);

  return (
    <nav className="flex w-16 flex-col items-center gap-1 border-r border-border bg-surface py-4">
      <div className="mb-4 flex h-9 w-9 items-center justify-center rounded-lg bg-gradient-to-br from-accent to-info text-sm font-bold text-white">
        VT
      </div>
      {NAV.map(({ view: v, label, icon: Icon }) => (
        <button
          key={v}
          onClick={() => setView(v)}
          title={label}
          className={clsx(
            "group flex h-12 w-12 flex-col items-center justify-center gap-0.5 rounded-xl transition-colors",
            view === v
              ? "bg-accent-soft text-accent"
              : "text-faint hover:bg-surface-2 hover:text-muted",
          )}
        >
          <Icon className="h-5 w-5" />
          <span className="text-[9px] font-medium">{label}</span>
        </button>
      ))}
    </nav>
  );
}
