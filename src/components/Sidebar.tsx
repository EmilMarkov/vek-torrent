// Левая навигация: только иконки, названия — во всплывающих подсказках.

import { clsx } from "clsx";
import { Download, Heart, History, Search, Settings } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { useAppStore, useCurrentRoute, type MainView } from "@/store";

const NAV: { view: MainView; label: string; icon: LucideIcon }[] = [
  { view: "search", label: "Поиск", icon: Search },
  { view: "downloads", label: "Загрузки", icon: Download },
  { view: "favorites", label: "Избранное", icon: Heart },
  { view: "history", label: "История", icon: History },
  { view: "settings", label: "Настройки", icon: Settings },
];

export function Sidebar() {
  const current = useCurrentRoute();
  const setView = useAppStore((s) => s.setView);
  const activeView = current.kind === "topic" ? null : current.kind;

  return (
    <nav className="flex w-14 flex-col items-center gap-1 border-r border-border bg-surface py-3">
      <div className="mb-3 flex h-9 w-9 items-center justify-center rounded-lg bg-gradient-to-br from-accent to-info text-sm font-bold text-white">
        VT
      </div>
      {NAV.map(({ view, label, icon: Icon }) => (
        <button
          key={view}
          onClick={() => setView(view)}
          title={label}
          aria-label={label}
          className={clsx(
            "group relative flex h-11 w-11 items-center justify-center rounded-xl transition-colors",
            activeView === view
              ? "bg-accent-soft text-accent"
              : "text-faint hover:bg-surface-2 hover:text-muted",
          )}
        >
          <Icon className="h-5 w-5" />
          {/* Всплывающая подсказка с названием раздела. */}
          <span className="pointer-events-none absolute left-full z-10 ml-2 rounded-md border border-border bg-surface-3 px-2 py-1 text-xs whitespace-nowrap text-text opacity-0 shadow-lg transition-opacity group-hover:opacity-100">
            {label}
          </span>
        </button>
      ))}
    </nav>
  );
}
