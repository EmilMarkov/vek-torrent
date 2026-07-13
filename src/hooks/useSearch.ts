// Lazy-search с серверной пагинацией: дебаунс запроса, отмена устаревших
// ответов, постраничная навигация (rutracker отдаёт по 50, максимум 500).
//
// Состояние хранится в zustand-сторе (а не в локальном useState страницы),
// поэтому переживает уход со страницы «Поиск» и возврат по кнопке «Назад» —
// запрос, результаты и позиция прокрутки сохраняются.

import { useCallback, useEffect } from "react";
import { create } from "zustand";

import { api, ApiError } from "@/lib/api";
import { DEFAULT_FILTERS, type ClientFilters } from "@/lib/filters";
import { pushSearchHistory } from "@/lib/searchHistory";
import type { SearchResult, SortField, SortOrder } from "@/lib/types";

const DEBOUNCE_MS = 450;

/** Размер серверной страницы rutracker. */
export const SEARCH_PAGE_SIZE = 50;
/** Максимум результатов, который отдаёт rutracker. */
const SERVER_CAP = 500;

interface SearchStore {
  query: string;
  author: string;
  /** Серверная сортировка rutracker (клик по заголовку таблицы). */
  sort: SortField;
  order: SortOrder;
  /** Результаты текущей страницы. */
  items: SearchResult[];
  totalFound: number;
  /** Текущая страница (с 1). */
  page: number;
  loading: boolean;
  error: ApiError | null;
  hasSearched: boolean;
  /** Ключ (запрос+автор) последнего успешного поиска: защита от повторного
   *  запуска того же поиска при возврате на страницу. */
  lastSearchedKey: string;
  /** Сохранённая позиция прокрутки списка результатов. */
  scrollTop: number;
  /** Клиентские фильтры (сохраняются между переходами). */
  filters: ClientFilters;

  setQuery: (query: string) => void;
  setAuthor: (author: string) => void;
  setScrollTop: (scrollTop: number) => void;
  setFilters: (filters: ClientFilters) => void;
}

// Служебные изменяемые значения вне рендера: токен продолжения, идентификатор
// актуального запроса (ответы с меньшим id отбрасываются — «отмена устаревших»)
// и таймер дебаунса (сбрасывается при немедленном поиске из setSort).
let runId = 0;
let searchId: string | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

const useSearchStore = create<SearchStore>((set) => ({
  query: "",
  author: "",
  sort: "seeders",
  order: "desc",
  items: [],
  totalFound: 0,
  page: 1,
  loading: false,
  error: null,
  hasSearched: false,
  lastSearchedKey: "",
  scrollTop: 0,
  filters: DEFAULT_FILTERS,
  setQuery: (query) => set({ query }),
  setAuthor: (author) => set({ author }),
  setScrollTop: (scrollTop) => set({ scrollTop }),
  setFilters: (filters) => set({ filters }),
}));

export function useSearch() {
  const store = useSearchStore();

  /**
   * Загружает страницу результатов. `newSession` начинает новую серверную
   * сессию поиска (rutracker фиксирует запрос и порядок в `search_id`,
   * поэтому смена запроса/сортировки требует сброса токена).
   *
   * Если страница внутри существующей сессии не загрузилась (токен истёк на
   * сервере), делается одна автоматическая попытка через новую сессию —
   * бэкенд умеет дочитать нужную страницу свежесозданной сессии.
   */
  const fetchPage = useCallback(async function run(
    targetPage: number,
    newSession: boolean,
  ): Promise<void> {
    const state = useSearchStore.getState();
    const currentQuery = state.query.trim();
    const currentAuthor = state.author.trim();
    if (!currentQuery && !currentAuthor) return;

    // Токен старой сессии не должен пережить смену запроса: иначе после
    // неудачного нового поиска пагинация подставит результаты старого.
    if (newSession) searchId = null;

    const id = ++runId;
    useSearchStore.setState({ loading: true, error: null, page: targetPage });

    try {
      const page = await api.search({
        query: currentQuery,
        forums: [],
        author: currentAuthor || null,
        sort: state.sort,
        order: state.order,
        offset: (targetPage - 1) * SEARCH_PAGE_SIZE,
        search_id: newSession ? null : searchId,
      });

      if (id !== runId) return; // устаревший ответ

      searchId = page.search_id;
      if (newSession && currentQuery) pushSearchHistory(currentQuery);
      useSearchStore.setState({
        items: page.items,
        totalFound: page.total_found,
        loading: false,
        error: null,
        hasSearched: true,
        lastSearchedKey: `${currentQuery}\n${currentAuthor}`,
        scrollTop: 0,
      });
    } catch (error) {
      if (id !== runId) return;
      // Сессия могла истечь на сервере — одна попытка с новой сессией.
      if (!newSession) {
        return run(targetPage, true);
      }
      useSearchStore.setState({
        loading: false,
        // Числа прошлого запроса не должны рисовать пагинатор поверх ошибки.
        items: [],
        totalFound: 0,
        error: error instanceof ApiError ? error : new ApiError("error", String(error)),
      });
    }
  }, []);

  // Дебаунс: новый поиск через паузу после последнего изменения запроса.
  useEffect(() => {
    const key = `${store.query.trim()}\n${store.author.trim()}`;
    if (!store.query.trim() && !store.author.trim()) {
      runId++;
      searchId = null;
      useSearchStore.setState({
        items: [],
        totalFound: 0,
        page: 1,
        loading: false,
        error: null,
        hasSearched: false,
        lastSearchedKey: "",
        scrollTop: 0,
      });
      return;
    }
    // Возврат на страницу с неизменным запросом: результаты, номер страницы
    // и позиция прокрутки уже в сторе — повторный поиск не нужен.
    if (key === useSearchStore.getState().lastSearchedKey) return;
    debounceTimer = setTimeout(() => void fetchPage(1, true), DEBOUNCE_MS);
    return () => clearTimeout(debounceTimer);
  }, [store.query, store.author, fetchPage]);

  // Смена сортировки — новая серверная сессия сразу, без дебаунса.
  // Взведённый таймер сбрасываем, чтобы не улетел дублирующий запрос.
  const setSort = useCallback(
    (sort: SortField, order: SortOrder) => {
      clearTimeout(debounceTimer);
      useSearchStore.setState({ sort, order });
      void fetchPage(1, true);
    },
    [fetchPage],
  );

  // Переход по страницам в рамках текущей серверной сессии.
  const setPage = useCallback(
    (page: number) => {
      void fetchPage(page, false);
    },
    [fetchPage],
  );

  const pageCount = Math.max(
    1,
    Math.ceil(Math.min(store.totalFound, SERVER_CAP) / SEARCH_PAGE_SIZE),
  );

  return {
    query: store.query,
    setQuery: store.setQuery,
    author: store.author,
    setAuthor: store.setAuthor,
    sort: store.sort,
    order: store.order,
    setSort,
    items: store.items,
    totalFound: store.totalFound,
    page: store.page,
    pageCount,
    setPage,
    loading: store.loading,
    error: store.error,
    hasSearched: store.hasSearched,
    scrollTop: store.scrollTop,
    setScrollTop: store.setScrollTop,
    filters: store.filters,
    setFilters: store.setFilters,
    retry: () => {
      const state = useSearchStore.getState();
      void fetchPage(state.page, state.page === 1);
    },
  };
}
