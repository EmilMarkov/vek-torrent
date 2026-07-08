// Левая навигация: только иконки, названия — во всплывающих подсказках.

import { clsx } from "clsx";
import { Download, Heart, History, Search, Settings } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { useDownloadsStore } from "@/hooks/useDownloads";
import { useFavoriteUpdatesCount } from "@/hooks/useLibrary";
import { useAppStore, useCurrentRoute, type MainView } from "@/store";

const NAV: { view: MainView; label: string; icon: LucideIcon }[] = [
  { view: "search", label: "Поиск", icon: Search },
  { view: "downloads", label: "Загрузки", icon: Download },
  { view: "favorites", label: "Избранное", icon: Heart },
  { view: "history", label: "История", icon: History },
  { view: "settings", label: "Настройки", icon: Settings },
];

/** Цвет индикатора загрузок по агрегированному статусу. */
function downloadsDotColor(states: string[]): string | null {
  if (states.length === 0) return null;
  if (states.includes("error")) return "bg-danger";
  if (states.includes("downloading")) return "bg-accent";
  if (states.includes("checking")) return "bg-warn";
  if (states.some((s) => s === "seeding" || s === "paused")) return "bg-success";
  return null;
}

export function Sidebar() {
  const current = useCurrentRoute();
  const setView = useAppStore((s) => s.setView);
  const activeView = current.kind === "topic" ? null : current.kind;
  const favoriteUpdates = useFavoriteUpdatesCount();
  const downloadsDot = downloadsDotColor(useDownloadsStore((s) => s.items).map((i) => i.state));

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
          {/* Индикатор обновлений избранного. */}
          {view === "favorites" && favoriteUpdates > 0 && (
            <span className="absolute top-1.5 right-1.5 h-2 w-2 rounded-full bg-accent ring-2 ring-surface" />
          )}
          {/* Индикатор состояния загрузок (цвет зависит от статуса). */}
          {view === "downloads" && downloadsDot && (
            <span
              className={clsx(
                "absolute top-1.5 right-1.5 h-2 w-2 rounded-full ring-2 ring-surface",
                downloadsDot,
              )}
            />
          )}
          {/* Всплывающая подсказка с названием раздела. */}
          <span className="pointer-events-none absolute left-full z-10 ml-2 rounded-md border border-border bg-surface-3 px-2 py-1 text-xs whitespace-nowrap text-text opacity-0 shadow-lg transition-opacity group-hover:opacity-100">
            {label}
          </span>
        </button>
      ))}
    </nav>
  );
}
