// История поисковых запросов (локально, в localStorage).

const KEY = "vek.search-history";
const MAX = 15;

export function loadSearchHistory(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

function save(history: string[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(history));
  } catch {
    // квота/приватный режим — история не критична
  }
}

/** Добавляет запрос в начало истории (дедуп, максимум MAX записей). */
export function pushSearchHistory(query: string) {
  const q = query.trim();
  if (!q) return;
  const history = loadSearchHistory().filter((h) => h !== q);
  history.unshift(q);
  save(history.slice(0, MAX));
}

export function removeSearchHistory(query: string): string[] {
  const history = loadSearchHistory().filter((h) => h !== query);
  save(history);
  return history;
}

export function clearSearchHistory(): string[] {
  save([]);
  return [];
}
