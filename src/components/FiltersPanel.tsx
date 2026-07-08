// Панель мгновенных клиентских фильтров результатов поиска.

import { X } from "lucide-react";

import { Input, Toggle } from "@/components/ui";
import {
  DEFAULT_FILTERS,
  hasActiveFilters,
  parseSizeInput,
  type ClientFilters,
  type ResultSortKey,
} from "@/lib/filters";

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

export function FiltersPanel({ filters, onChange }: Props) {
  const update = (patch: Partial<ClientFilters>) => onChange({ ...filters, ...patch });

  return (
    <div className="grid grid-cols-2 gap-x-4 gap-y-3 rounded-lg border border-border bg-surface-2/50 p-4 md:grid-cols-4">
      <Field label="Уточнить (−слово исключает)">
        <Input
          value={filters.refine}
          onChange={(e) => update({ refine: e.target.value })}
          placeholder="напр. 1080p -ts"
        />
      </Field>

      <Field label="Размер от">
        <Input
          defaultValue=""
          onChange={(e) => update({ minSizeBytes: parseSizeInput(e.target.value) })}
          placeholder="напр. 700мб"
        />
      </Field>

      <Field label="Размер до">
        <Input
          defaultValue=""
          onChange={(e) => update({ maxSizeBytes: parseSizeInput(e.target.value) })}
          placeholder="напр. 20гб"
        />
      </Field>

      <Field label="Сидов не меньше">
        <Input
          type="number"
          min={0}
          defaultValue=""
          onChange={(e) => update({ minSeeders: e.target.value ? Number(e.target.value) : null })}
          placeholder="напр. 5"
        />
      </Field>

      <Field label="Сортировка">
        <select
          value={filters.sortKey}
          onChange={(e) => update({ sortKey: e.target.value as ResultSortKey })}
          className="w-full rounded-lg border border-border bg-surface-2 px-3 py-2 text-sm text-text focus:border-accent/70 focus:outline-none"
        >
          {SORT_OPTIONS.map((o) => (
            <option key={o.key} value={o.key}>
              {o.label}
            </option>
          ))}
        </select>
      </Field>

      <Field label="Порядок">
        <button
          onClick={() => update({ sortDesc: !filters.sortDesc })}
          disabled={filters.sortKey === "relevance"}
          className="w-full rounded-lg border border-border bg-surface-2 px-3 py-2 text-sm text-text hover:border-border-strong disabled:opacity-40"
        >
          {filters.sortDesc ? "По убыванию ↓" : "По возрастанию ↑"}
        </button>
      </Field>

      <div className="flex items-end">
        <Toggle
          checked={filters.onlyApproved}
          onChange={(v) => update({ onlyApproved: v })}
          label="Только проверенные"
        />
      </div>

      <div className="flex items-end">
        {hasActiveFilters(filters) && (
          <button
            onClick={() => onChange({ ...DEFAULT_FILTERS })}
            className="inline-flex items-center gap-1 text-xs text-faint hover:text-danger"
          >
            <X className="h-3.5 w-3.5" />
            Сбросить фильтры
          </button>
        )}
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
