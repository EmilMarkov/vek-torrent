// Дерево разделов rutracker с чекбоксами и поиском (переиспользуется
// фильтрами поиска и редактором категорий).

import { useQuery } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { useState } from "react";

import { Input } from "@/components/ui";
import { api } from "@/lib/api";
import type { ForumGroup } from "@/lib/types";

/** Дерево разделов rutracker (загружается после входа, кэшируется). */
export function useForumGroups() {
  return useQuery({
    queryKey: ["categories"],
    queryFn: api.categories,
    staleTime: 10 * 60_000,
    retry: false,
  });
}

interface Props {
  selected: number[];
  onChange: (ids: number[]) => void;
  /** Заголовок над деревом (по умолчанию «Раздел»). */
  label?: string;
}

export function ForumTreePicker({ selected, onChange, label = "Раздел" }: Props) {
  const categories = useForumGroups();
  const [query, setQuery] = useState("");

  const groups = categories.data ?? null;
  const selectedSet = new Set(selected);
  const toggle = (id: number) => {
    const next = new Set(selectedSet);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    onChange([...next]);
  };

  const q = query.trim().toLowerCase();
  const visible = filterGroups(groups ?? [], q);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-[11px] font-medium text-faint">{label}</span>
        {selected.length > 0 && (
          <button onClick={() => onChange([])} className="text-[11px] text-faint hover:text-danger">
            снять ({selected.length})
          </button>
        )}
      </div>

      {categories.isLoading ? (
        <p className="text-xs text-faint">Загрузка разделов…</p>
      ) : categories.isError || !groups ? (
        <p className="text-xs text-faint">
          Разделы доступны после входа на rutracker (см. Настройки).
        </p>
      ) : (
        <>
          <div className="relative">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 h-3.5 w-3.5 -translate-y-1/2 text-faint" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Найти раздел…"
              className="pl-8 text-xs"
            />
          </div>
          <div className="max-h-64 overflow-y-auto rounded-lg border border-border bg-surface-2/50">
            {visible.length === 0 ? (
              <p className="px-3 py-2 text-xs text-faint">Ничего не найдено</p>
            ) : (
              visible.map((group) => (
                <div key={group.title}>
                  <div className="sticky top-0 bg-surface-3/90 px-2.5 py-1 text-[10px] font-semibold tracking-wide text-muted uppercase backdrop-blur">
                    {group.title}
                  </div>
                  {group.forums.map((forum) => (
                    <label
                      key={forum.id}
                      className="flex cursor-pointer items-center gap-2 px-2.5 py-1.5 text-xs text-text hover:bg-surface-3"
                      style={{ paddingLeft: `${10 + forum.depth * 12}px` }}
                    >
                      <input
                        type="checkbox"
                        checked={selectedSet.has(forum.id)}
                        onChange={() => toggle(forum.id)}
                        className="accent-accent"
                      />
                      <span className="truncate">{forum.name}</span>
                    </label>
                  ))}
                </div>
              ))
            )}
          </div>
        </>
      )}
    </div>
  );
}

/** Фильтрует группы форумов по подстроке (оставляя непустые группы). */
function filterGroups(groups: ForumGroup[], query: string): ForumGroup[] {
  if (!query) return groups;
  return groups
    .map((g) => ({ ...g, forums: g.forums.filter((f) => f.name.toLowerCase().includes(query)) }))
    .filter((g) => g.forums.length > 0);
}
