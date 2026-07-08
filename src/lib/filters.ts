// Мгновенная клиентская фильтрация уже загруженных результатов поиска —
// ключевая доработка над rutracker: без повторных запросов к трекеру.

import type { SearchResult } from "./types";

export type ResultSortKey = "relevance" | "seeders" | "size" | "downloads" | "date" | "title";

export interface ClientFilters {
  /** Строка «живого» уточнения: термины через пробел, `-слово` — исключение. */
  refine: string;
  minSizeBytes: number | null;
  maxSizeBytes: number | null;
  minSeeders: number | null;
  /** Только проверенные модератором. */
  onlyApproved: boolean;
  /** Ограничение по форумам (id); пусто — без ограничения. */
  forumIds: number[];
  sortKey: ResultSortKey;
  sortDesc: boolean;
}

export const DEFAULT_FILTERS: ClientFilters = {
  refine: "",
  minSizeBytes: null,
  maxSizeBytes: null,
  minSeeders: null,
  onlyApproved: false,
  forumIds: [],
  sortKey: "relevance",
  sortDesc: true,
};

interface ParsedRefine {
  include: string[];
  exclude: string[];
}

/** Разбирает строку уточнения на обязательные и исключающие термины. */
export function parseRefine(refine: string): ParsedRefine {
  const include: string[] = [];
  const exclude: string[] = [];
  for (const raw of refine.toLowerCase().split(/\s+/)) {
    const term = raw.trim();
    if (!term) continue;
    if (term.startsWith("-")) {
      // Одинокий «-» игнорируем; иначе это исключающий термин.
      if (term.length > 1) exclude.push(term.slice(1));
    } else {
      include.push(term);
    }
  }
  return { include, exclude };
}

function matchesRefine(title: string, parsed: ParsedRefine): boolean {
  const lower = title.toLowerCase();
  return (
    parsed.include.every((term) => lower.includes(term)) &&
    !parsed.exclude.some((term) => lower.includes(term))
  );
}

function compare(a: SearchResult, b: SearchResult, key: ResultSortKey): number {
  switch (key) {
    case "seeders":
      return a.seeders - b.seeders;
    case "size":
      return a.size_bytes - b.size_bytes;
    case "downloads":
      return a.downloads - b.downloads;
    case "date":
      return a.added_unix - b.added_unix;
    case "title":
      return a.title.localeCompare(b.title, "ru");
    case "relevance":
      return 0;
  }
}

/** Применяет фильтры и сортировку к списку результатов (чистая функция). */
export function applyFilters(items: SearchResult[], filters: ClientFilters): SearchResult[] {
  const parsed = parseRefine(filters.refine);
  const forumSet = new Set(filters.forumIds);

  const filtered = items.filter((item) => {
    if (!matchesRefine(item.title, parsed)) return false;
    if (filters.minSizeBytes !== null && item.size_bytes < filters.minSizeBytes) return false;
    if (filters.maxSizeBytes !== null && item.size_bytes > filters.maxSizeBytes) return false;
    if (filters.minSeeders !== null && item.seeders < filters.minSeeders) return false;
    if (filters.onlyApproved && item.approval !== "approved") return false;
    if (forumSet.size > 0 && (item.forum === null || !forumSet.has(item.forum.id))) return false;
    return true;
  });

  if (filters.sortKey !== "relevance") {
    const dir = filters.sortDesc ? -1 : 1;
    // Стабильная сортировка: сохраняем исходный порядок при равенстве.
    filtered
      .map((item, index) => ({ item, index }))
      .sort((a, b) => {
        const c = compare(a.item, b.item, filters.sortKey);
        return c !== 0 ? c * dir : a.index - b.index;
      })
      .forEach(({ item }, i) => {
        filtered[i] = item;
      });
  }

  return filtered;
}

/** Есть ли активные (непустые) фильтры — для индикации в UI. */
export function hasActiveFilters(filters: ClientFilters): boolean {
  return (
    filters.refine.trim() !== "" ||
    filters.minSizeBytes !== null ||
    filters.maxSizeBytes !== null ||
    filters.minSeeders !== null ||
    filters.onlyApproved ||
    filters.forumIds.length > 0
  );
}

/** Парсит человеко-размер («1.5 гб», «700мб») в байты для полей фильтра. */
export function parseSizeInput(input: string): number | null {
  const trimmed = input.trim().toLowerCase().replace(",", ".");
  if (!trimmed) return null;
  const match = trimmed.match(/^([\d.]+)\s*(б|кб|мб|гб|тб|b|kb|mb|gb|tb)?$/);
  if (!match) return null;
  const value = parseFloat(match[1]);
  if (!Number.isFinite(value)) return null;
  const unit = match[2] ?? "мб";
  const factor: Record<string, number> = {
    б: 1,
    b: 1,
    кб: 1024,
    kb: 1024,
    мб: 1024 ** 2,
    mb: 1024 ** 2,
    гб: 1024 ** 3,
    gb: 1024 ** 3,
    тб: 1024 ** 4,
    tb: 1024 ** 4,
  };
  return Math.round(value * (factor[unit] ?? 1024 ** 2));
}
