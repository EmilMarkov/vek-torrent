// Правый сайдбар фильтров результатов поиска (мгновенная клиентская фильтрация).

import { useQuery } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, Search, X } from "lucide-react";
import { useState } from "react";

import { Input, Select, Toggle } from "@/components/ui";
import { api } from "@/lib/api";
import {
  DEFAULT_FILTERS,
  forumIdsForCategory,
  GENERAL_CATEGORIES,
  hasActiveFilters,
  parseSizeInput,
  type ClientFilters,
  type ResultSortKey,
} from "@/lib/filters";
import type { ForumGroup } from "@/lib/types";

const SORT_OPTIONS: { value: ResultSortKey; label: string }[] = [
  { value: "relevance", label: "По релевантности" },
  { value: "seeders", label: "По сидам" },
  { value: "size", label: "По размеру" },
  { value: "downloads", label: "По скачиваниям" },
  { value: "date", label: "По дате" },
  { value: "title", label: "По названию" },
];

interface Props {
  filters: ClientFilters;
  onChange: (filters: ClientFilters) => void;
}

/** Компактное представление размера в байтах для поля ввода фильтра. */
function bytesToInput(bytes: number | null): string {
  if (bytes === null) return "";
  const units: [number, string][] = [
    [1024 ** 4, "тб"],
    [1024 ** 3, "гб"],
    [1024 ** 2, "мб"],
    [1024, "кб"],
  ];
  for (const [factor, unit] of units) {
    if (bytes >= factor) {
      const value = bytes / factor;
      return `${Number.isInteger(value) ? value : value.toFixed(1)}${unit}`;
    }
  }
  return String(bytes);
}

export function FiltersSidebar({ filters, onChange }: Props) {
  // Инициализируем поля из сохранённых фильтров (состояние переживает переходы).
  const [sizeMin, setSizeMin] = useState(() => bytesToInput(filters.minSizeBytes));
  const [sizeMax, setSizeMax] = useState(() => bytesToInput(filters.maxSizeBytes));
  const [seeders, setSeeders] = useState(() =>
    filters.minSeeders === null ? "" : String(filters.minSeeders),
  );

  const categories = useQuery({
    queryKey: ["categories"],
    queryFn: api.categories,
    staleTime: 10 * 60_000,
    retry: false,
  });

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
        <Field label={'Уточнить (−слово, −"фраза" исключают)'}>
          <Input
            value={filters.refine}
            onChange={(e) => update({ refine: e.target.value })}
            placeholder={'напр. 1080p -ts -"Fallout Shelter"'}
          />
        </Field>

        <Field label="Автор">
          <Input
            value={filters.author}
            onChange={(e) => update({ author: e.target.value })}
            placeholder="имя автора раздачи"
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

        <Field label="Сортировка">
          <div className="flex items-center gap-2">
            <Select
              className="flex-1"
              value={filters.sortKey}
              onChange={(sortKey) => update({ sortKey })}
              options={SORT_OPTIONS}
            />
            <button
              onClick={() => update({ sortDesc: !filters.sortDesc })}
              disabled={filters.sortKey === "relevance"}
              title={filters.sortDesc ? "По убыванию" : "По возрастанию"}
              className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-border bg-surface-2 text-muted hover:border-border-strong hover:text-text disabled:opacity-40"
            >
              {filters.sortDesc ? (
                <ArrowDown className="h-4 w-4" />
              ) : (
                <ArrowUp className="h-4 w-4" />
              )}
            </button>
          </div>
        </Field>

        <Toggle
          checked={filters.onlyApproved}
          onChange={(v) => update({ onlyApproved: v })}
          label="Только проверенные"
        />

        <CategoryChips
          groups={categories.data ?? []}
          forumIds={filters.forumIds}
          onChange={(forumIds) => update({ forumIds })}
        />

        <ForumFilter
          groups={categories.data ?? null}
          loading={categories.isLoading}
          error={categories.isError}
          selected={filters.forumIds}
          onChange={(forumIds) => update({ forumIds })}
        />
      </div>
    </aside>
  );
}

/** Чекбоксы обобщённых категорий: выбирают все подходящие разделы rutracker. */
function CategoryChips({
  groups,
  forumIds,
  onChange,
}: {
  groups: ForumGroup[];
  forumIds: number[];
  onChange: (ids: number[]) => void;
}) {
  if (groups.length === 0) return null;
  const selected = new Set(forumIds);

  const toggle = (categoryKey: string) => {
    const category = GENERAL_CATEGORIES.find((c) => c.key === categoryKey);
    if (!category) return;
    const ids = forumIdsForCategory(groups, category);
    if (ids.length === 0) return;
    const active = ids.every((id) => selected.has(id));
    const next = new Set(selected);
    for (const id of ids) {
      if (active) next.delete(id);
      else next.add(id);
    }
    onChange([...next]);
  };

  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-[11px] font-medium text-faint">Категория</span>
      <div className="flex flex-wrap gap-1.5">
        {GENERAL_CATEGORIES.map((category) => {
          const ids = forumIdsForCategory(groups, category);
          const active = ids.length > 0 && ids.every((id) => selected.has(id));
          return (
            <button
              key={category.key}
              onClick={() => toggle(category.key)}
              disabled={ids.length === 0}
              className={
                active
                  ? "rounded-full bg-accent-soft px-3 py-1 text-xs font-medium text-accent"
                  : "rounded-full border border-border px-3 py-1 text-xs text-muted hover:border-border-strong hover:text-text disabled:opacity-40"
              }
            >
              {category.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function ForumFilter({
  groups,
  loading,
  error,
  selected,
  onChange,
}: {
  groups: ForumGroup[] | null;
  loading: boolean;
  error: boolean;
  selected: number[];
  onChange: (ids: number[]) => void;
}) {
  const [query, setQuery] = useState("");

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
        <span className="text-[11px] font-medium text-faint">Раздел</span>
        {selected.length > 0 && (
          <button onClick={() => onChange([])} className="text-[11px] text-faint hover:text-danger">
            снять ({selected.length})
          </button>
        )}
      </div>

      {loading ? (
        <p className="text-xs text-faint">Загрузка разделов…</p>
      ) : error || !groups ? (
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-[11px] font-medium text-faint">{label}</span>
      {children}
    </label>
  );
}
