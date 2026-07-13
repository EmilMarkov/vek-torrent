// Правый сайдбар фильтров результатов поиска (мгновенная клиентская фильтрация).

import { useQuery } from "@tanstack/react-query";
import { X } from "lucide-react";
import { useState } from "react";

import { ForumTreePicker, useForumGroups } from "@/components/ForumTreePicker";
import { Input, Toggle } from "@/components/ui";
import { api } from "@/lib/api";
import {
  DEFAULT_FILTERS,
  effectiveCategoryForumIds,
  hasActiveFilters,
  parseSizeInput,
  type ClientFilters,
} from "@/lib/filters";
import type { CategoryItem, ForumGroup } from "@/lib/types";

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

  const forumGroups = useForumGroups();
  const { data: userCategories } = useQuery({
    queryKey: ["user-categories"],
    queryFn: api.userCategories,
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

        <Toggle
          checked={filters.onlyApproved}
          onChange={(v) => update({ onlyApproved: v })}
          label="Только проверенные"
        />

        <CategoryChips
          categories={userCategories ?? []}
          groups={forumGroups.data ?? []}
          forumIds={filters.forumIds}
          onChange={(forumIds) => update({ forumIds })}
        />

        <ForumTreePicker
          selected={filters.forumIds}
          onChange={(forumIds) => update({ forumIds })}
        />
      </div>
    </aside>
  );
}

/**
 * Чипы пользовательских категорий: выбирают все разделы rutracker категории.
 * Наборы разделов настраиваются на странице «Категории».
 */
function CategoryChips({
  categories,
  groups,
  forumIds,
  onChange,
}: {
  categories: CategoryItem[];
  groups: ForumGroup[];
  forumIds: number[];
  onChange: (ids: number[]) => void;
}) {
  if (categories.length === 0) return null;
  const selected = new Set(forumIds);

  const idsOf = (category: CategoryItem) => effectiveCategoryForumIds(groups, category);

  const toggle = (category: CategoryItem) => {
    const ids = idsOf(category);
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
        {categories.map((category) => {
          const ids = idsOf(category);
          const active = ids.length > 0 && ids.every((id) => selected.has(id));
          return (
            <button
              key={category.id}
              onClick={() => toggle(category)}
              disabled={ids.length === 0}
              title={
                ids.length === 0
                  ? "У категории не настроены разделы (страница «Категории»)"
                  : `Разделов: ${ids.length}`
              }
              className={
                active
                  ? "rounded-full px-3 py-1 text-xs font-medium"
                  : "rounded-full border border-border px-3 py-1 text-xs text-muted hover:border-border-strong hover:text-text disabled:opacity-40"
              }
              style={
                active
                  ? { backgroundColor: `${category.color}26`, color: category.color }
                  : undefined
              }
            >
              {category.name}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-[11px] font-medium text-faint">{label}</span>
      {children}
    </label>
  );
}
