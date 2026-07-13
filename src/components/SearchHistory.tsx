// Выпадающая история поисковых запросов под полем ввода.

import { History, X } from "lucide-react";
import { useState } from "react";

import { clearSearchHistory, loadSearchHistory, removeSearchHistory } from "@/lib/searchHistory";

interface Props {
  /** Текущий ввод — история фильтруется по подстроке. */
  query: string;
  onPick: (query: string) => void;
  onClose: () => void;
}

export function SearchHistoryDropdown({ query, onPick, onClose }: Props) {
  const [history, setHistory] = useState(loadSearchHistory);

  const q = query.trim().toLowerCase();
  const visible = history.filter((h) => h.toLowerCase().includes(q) && h !== query.trim());
  if (visible.length === 0) return null;

  return (
    <div className="absolute top-full right-0 left-0 z-30 mt-1 overflow-hidden rounded-lg border border-border bg-surface-3 py-1 shadow-lg">
      {visible.slice(0, 8).map((item) => (
        <div key={item} className="group flex items-center hover:bg-surface-2">
          {/* mousedown, а не click: клик по элементу не должен сначала
              закрыть список через blur поля ввода. */}
          <button
            onMouseDown={(e) => {
              e.preventDefault();
              onPick(item);
              onClose();
            }}
            className="flex min-w-0 flex-1 items-center gap-2 px-3 py-1.5 text-left text-sm text-text"
          >
            <History className="h-3.5 w-3.5 shrink-0 text-faint" />
            <span className="truncate">{item}</span>
          </button>
          <button
            onMouseDown={(e) => {
              e.preventDefault();
              setHistory(removeSearchHistory(item));
            }}
            title="Убрать из истории"
            className="px-2 py-1.5 text-faint opacity-0 group-hover:opacity-100 hover:text-danger"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      ))}
      <div className="border-t border-border/60 px-3 py-1 text-right">
        <button
          onMouseDown={(e) => {
            e.preventDefault();
            setHistory(clearSearchHistory());
          }}
          className="text-[11px] text-faint hover:text-danger"
        >
          очистить историю
        </button>
      </div>
    </div>
  );
}
