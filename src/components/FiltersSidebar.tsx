// Правый сайдбар фильтров результатов поиска (мгновенная клиентская фильтрация).

import { useQuery } from "@tanstack/react-query";
import { Search, X } from "lucide-react";
import { useState } from "react";

import { Input, Toggle } from "@/components/ui";
import { api } from "@/lib/api";
import {
  DEFAULT_FILTERS,
  hasActiveFilters,
  parseSizeInput,
  type ClientFilters,
  type ResultSortKey,
} from "@/lib/filters";
import type { ForumGroup } from "@/lib/types";

const SORT_OPTIONS: { key: ResultSortKey; label: string }[] = [
  { key: "relevance", label: "По релевантности" },
  { key: "seeders", label: "По сидам" },
  { key: "size", label: "По размеру" },
  { key: "downloads", label: "По скачиваниям" },
  { key: "date", label: "По дате" },
  { key: "title", label: "По названию" },
];

interface Props {
  filters: ClientFilters;
  onChange: (filters: ClientFilters) => void;
}

const selectClass =
  "w-full rounded-lg border border-border bg-surface-2 px-3 py-2 text-sm text-text focus:border-accent/70 focus:outline-none";

export function FiltersSidebar({ filters, onChange }: Props) {
  const [sizeMin, setSizeMin] = useState("");
  const [sizeMax, setSizeMax] = useState("");
  const [seeders, setSeeders] = useState("");

  const update = (patch: Partial<ClientFilters>) => onChange({ ...filters, ...patch });

  const reset = () => {
    setSizeMin("");
    setSizeMax("");
    setSeeders("");
    onChange({ ...DEFAULT_FILTERS });
  };

  return (
    <aside className="flex w-72 shrink-0 flex-col overflow-y-auto border-l border-border bg-surface/40">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <span className="text-sm font-semibold text-text">Фильтры</span>
        {hasActiveFilters(filters) && (
          <button
            onClick={reset}
            className="inline-flex items-center gap-1 text-xs text-faint hover:text-danger"
          >
            <X className="h-3.5 w-3.5" />
            Сбросить
          </button>
        )}
      </div>

      <div className="flex flex-col gap-4 p-4">
        <Field label="Уточнить (−слово исключает)">
          <Input
            value={filters.refine}
            onChange={(e) => update({ refine: e.target.value })}
            placeholder="напр. 1080p -ts"
          />
        </Field>

        <div className="grid grid-cols-2 gap-2">
          <Field label="Размер от">
            <Input
              value={sizeMin}
              onChange={(e) => {
                setSizeMin(e.target.value);
                update({ minSizeBytes: parseSizeInput(e.target.value) });
              }}
              placeholder="700мб"
            />
          </Field>
          <Field label="Размер до">
            <Input
              value={sizeMax}
              onChange={(e) => {
                setSizeMax(e.target.value);
                update({ maxSizeBytes: parseSizeInput(e.target.value) });
              }}
              placeholder="20гб"
            />
          </Field>
        </div>

        <Field label="Сидов не меньше">
          <Input
            type="number"
            min={0}
            value={seeders}
            onChange={(e) => {
              setSeeders(e.target.value);
              update({ minSeeders: e.target.value ? Number(e.target.value) : null });
            }}
            placeholder="напр. 5"
          />
        </Field>

        <div className="grid grid-cols-1 gap-2">
          <Field label="Сортировка">
            <select
              value={filters.sortKey}
              onChange={(e) => update({ sortKey: e.target.value as ResultSortKey })}
              className={selectClass}
            >
              {SORT_OPTIONS.map((o) => (
                <option key={o.key} value={o.key}>
                  {o.label}
                </option>
              ))}
            </select>
          </Field>
          <button
            onClick={() => update({ sortDesc: !filters.sortDesc })}
            disabled={filters.sortKey === "relevance"}
            className={`${selectClass} text-left hover:border-border-strong disabled:opacity-40`}
          >
            {filters.sortDesc ? "По убыванию ↓" : "По возрастанию ↑"}
          </button>
        </div>

        <Toggle
          checked={filters.onlyApproved}
          onChange={(v) => update({ onlyApproved: v })}
          label="Только проверенные"
        />

        <ForumFilter selected={filters.forumIds} onChange={(forumIds) => update({ forumIds })} />
      </div>
    </aside>
  );
}

function ForumFilter({
  selected,
  onChange,
}: {
  selected: number[];
  onChange: (ids: number[]) => void;
}) {
  const [query, setQuery] = useState("");
  const { data, isLoading, isError } = useQuery({
    queryKey: ["categories"],
    queryFn: api.categories,
    staleTime: 10 * 60_000,
    retry: false,
  });

  const selectedSet = new Set(selected);
  const toggle = (id: number) => {
    const next = new Set(selectedSet);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    onChange([...next]);
  };

  const q = query.trim().toLowerCase();
  const groups = filterGroups(data ?? [], q);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-[11px] font-medium text-faint">Раздел</span>
        {selected.length > 0 && (
          <button onClick={() => onChange([])} className="text-[11px] text-faint hover:text-danger">
            снять ({selected.length})
          </button>
        )}
      </div>

      {isLoading ? (
        <p className="text-xs text-faint">Загрузка разделов…</p>
      ) : isError || !data ? (
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
            {groups.length === 0 ? (
              <p className="px-3 py-2 text-xs text-faint">Ничего не найдено</p>
            ) : (
              groups.map((group) => (
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-[11px] font-medium text-faint">{label}</span>
      {children}
    </label>
  );
}
