// Мгновенная клиентская фильтрация уже загруженных результатов поиска —
// ключевая доработка над rutracker: без повторных запросов к трекеру.

import type { ForumGroup, SearchResult } from "./types";

export interface ClientFilters {
  /**
   * Строка «живого» уточнения. Поддерживает слова и словосочетания в кавычках,
   * а также исключение через `-`: `1080p -ts -"Fallout Shelter"`.
   */
  refine: string;
  /** Фильтр по автору раздачи (подстрока). */
  author: string;
  minSizeBytes: number | null;
  maxSizeBytes: number | null;
  minSeeders: number | null;
  /** Только проверенные модератором. */
  onlyApproved: boolean;
  /** Ограничение по форумам (id); пусто — без ограничения. */
  forumIds: number[];
}

export const DEFAULT_FILTERS: ClientFilters = {
  refine: "",
  author: "",
  minSizeBytes: null,
  maxSizeBytes: null,
  minSeeders: null,
  onlyApproved: false,
  forumIds: [],
};

interface ParsedRefine {
  include: string[];
  exclude: string[];
}

/**
 * Разбирает строку уточнения на обязательные и исключающие термины.
 *
 * Поддерживает словосочетания в двойных кавычках (в т.ч. с `-` для исключения):
 * `linux "linux mint" -beta -"release candidate"`.
 */
export function parseRefine(refine: string): ParsedRefine {
  const include: string[] = [];
  const exclude: string[] = [];
  const text = refine.toLowerCase();
  let i = 0;

  while (i < text.length) {
    while (i < text.length && /\s/.test(text[i])) i++;
    if (i >= text.length) break;

    let negative = false;
    if (text[i] === "-") {
      negative = true;
      i++;
    }

    let term = "";
    if (text[i] === '"') {
      i++; // пропускаем открывающую кавычку
      while (i < text.length && text[i] !== '"') term += text[i++];
      if (i < text.length) i++; // пропускаем закрывающую кавычку
    } else {
      while (i < text.length && !/\s/.test(text[i])) term += text[i++];
    }

    term = term.trim();
    if (!term) continue; // одинокий «-» или пустые кавычки
    (negative ? exclude : include).push(term);
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

/**
 * Применяет фильтры к списку результатов (чистая функция).
 *
 * Порядок результатов не меняется: сортировка — серверная (rutracker),
 * управляется кликами по заголовкам таблицы и приходит уже отсортированной.
 */
export function applyFilters(items: SearchResult[], filters: ClientFilters): SearchResult[] {
  const parsed = parseRefine(filters.refine);
  const forumSet = new Set(filters.forumIds);
  const author = filters.author.trim().toLowerCase();

  return items.filter((item) => {
    if (!matchesRefine(item.title, parsed)) return false;
    if (author && !(item.author ?? "").toLowerCase().includes(author)) return false;
    if (filters.minSizeBytes !== null && item.size_bytes < filters.minSizeBytes) return false;
    if (filters.maxSizeBytes !== null && item.size_bytes > filters.maxSizeBytes) return false;
    if (filters.minSeeders !== null && item.seeders < filters.minSeeders) return false;
    if (filters.onlyApproved && item.approval !== "approved") return false;
    if (forumSet.size > 0 && (item.forum === null || !forumSet.has(item.forum.id))) return false;
    return true;
  });
}

/** Есть ли активные (непустые) фильтры — для индикации в UI. */
export function hasActiveFilters(filters: ClientFilters): boolean {
  return (
    filters.refine.trim() !== "" ||
    filters.author.trim() !== "" ||
    filters.minSizeBytes !== null ||
    filters.maxSizeBytes !== null ||
    filters.minSeeders !== null ||
    filters.onlyApproved ||
    filters.forumIds.length > 0
  );
}

/** Обобщённая категория и ключевые слова для сопоставления с разделами. */
export interface GeneralCategory {
  key: string;
  label: string;
  test: RegExp;
}

/**
 * Обобщённые категории. Раздел rutracker относится к категории, если его имя
 * или название группы совпадает с ключевыми словами. Книги проверяются раньше
 * музыки, чтобы «аудиокниги» не попадали в музыку.
 */
export const GENERAL_CATEGORIES: GeneralCategory[] = [
  { key: "films", label: "Фильмы", test: /кино|фильм|сериал|мультфильм|мультсериал/i },
  { key: "books", label: "Книги", test: /книг|литератур/i },
  { key: "music", label: "Музыка", test: /музык|песн|альбом|дискограф/i },
  { key: "games", label: "Игры", test: /игр|game|консол/i },
];

/** Возвращает id разделов, попадающих под обобщённую категорию. */
export function forumIdsForCategory(groups: ForumGroup[], category: GeneralCategory): number[] {
  const ids: number[] = [];
  for (const group of groups) {
    const groupMatches = category.test.test(group.title);
    for (const forum of group.forums) {
      if (groupMatches || category.test.test(forum.name)) ids.push(forum.id);
    }
  }
  return ids;
}

/**
 * Эффективные разделы пользовательской категории: заданные явно, а для
 * стандартных категорий без настройки — эвристика по ключевым словам
 * (как раньше вели себя встроенные «Фильмы/Книги/Музыка/Игры»).
 */
export function effectiveCategoryForumIds(
  groups: ForumGroup[],
  category: { name: string; forumIds: number[] },
): number[] {
  if (category.forumIds.length > 0) return category.forumIds;
  const general = GENERAL_CATEGORIES.find(
    (g) => g.label.toLowerCase() === category.name.trim().toLowerCase(),
  );
  return general ? forumIdsForCategory(groups, general) : [];
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
