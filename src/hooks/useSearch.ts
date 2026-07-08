// Lazy-search: дебаунс запроса, отмена устаревших ответов, догрузка страниц.
//
// Состояние хранится в zustand-сторе (а не в локальном useState страницы),
// поэтому переживает уход со страницы «Поиск» и возврат по кнопке «Назад» —
// запрос, результаты и позиция прокрутки сохраняются (требование п.3).

import { useCallback, useEffect } from "react";
import { create } from "zustand";

import { api, ApiError } from "@/lib/api";
import { DEFAULT_FILTERS, type ClientFilters } from "@/lib/filters";
import type { SearchResult } from "@/lib/types";

const DEBOUNCE_MS = 450;

interface SearchStore {
  query: string;
  author: string;
  items: SearchResult[];
  totalFound: number;
  loading: boolean;
  loadingMore: boolean;
  error: ApiError | null;
  hasSearched: boolean;
  hasMore: boolean;
  /** Сохранённая позиция прокрутки списка результатов. */
  scrollTop: number;
  /** Клиентские фильтры (сохраняются между переходами). */
  filters: ClientFilters;

  setQuery: (query: string) => void;
  setAuthor: (author: string) => void;
  setScrollTop: (scrollTop: number) => void;
  setFilters: (filters: ClientFilters) => void;
}

// Служебные изменяемые значения вне рендера: токен продолжения и идентификатор
// актуального запроса (ответы с меньшим id отбрасываются — «отмена устаревших»).
let runId = 0;
let searchId: string | null = null;

const useSearchStore = create<SearchStore>((set) => ({
  query: "",
  author: "",
  items: [],
  totalFound: 0,
  loading: false,
  loadingMore: false,
  error: null,
  hasSearched: false,
  hasMore: false,
  scrollTop: 0,
  filters: DEFAULT_FILTERS,
  setQuery: (query) => set({ query }),
  setAuthor: (author) => set({ author }),
  setScrollTop: (scrollTop) => set({ scrollTop }),
  setFilters: (filters) => set({ filters }),
}));

export function useSearch() {
  const store = useSearchStore();

  const performSearch = useCallback(async (reset: boolean) => {
    const state = useSearchStore.getState();
    const currentQuery = state.query.trim();
    const currentAuthor = state.author.trim();
    if (!currentQuery && !currentAuthor) return;

    const id = ++runId;
    useSearchStore.setState({ loading: reset, loadingMore: !reset, error: null });

    try {
      const offset = reset ? 0 : useSearchStore.getState().items.length;
      const page = await api.search({
        query: currentQuery,
        forums: [],
        author: currentAuthor || null,
        sort: "seeders",
        order: "desc",
        offset,
        search_id: reset ? null : searchId,
      });

      if (id !== runId) return; // устаревший ответ

      searchId = page.search_id;
      const prev = useSearchStore.getState().items;
      const items = reset ? page.items : [...prev, ...page.items];
      useSearchStore.setState({
        items,
        totalFound: page.total_found,
        loading: false,
        loadingMore: false,
        error: null,
        hasSearched: true,
        hasMore: items.length < page.total_found && page.items.length > 0,
        ...(reset ? { scrollTop: 0 } : {}),
      });
    } catch (error) {
      if (id !== runId) return;
      useSearchStore.setState({
        loading: false,
        loadingMore: false,
        error: error instanceof ApiError ? error : new ApiError("error", String(error)),
      });
    }
  }, []);

  // Дебаунс: новый поиск через паузу после последнего изменения запроса.
  useEffect(() => {
    if (!store.query.trim() && !store.author.trim()) {
      runId++;
      searchId = null;
      useSearchStore.setState({
        items: [],
        totalFound: 0,
        loading: false,
        loadingMore: false,
        error: null,
        hasSearched: false,
        hasMore: false,
        scrollTop: 0,
      });
      return;
    }
    const timer = setTimeout(() => void performSearch(true), DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [store.query, store.author, performSearch]);

  const loadMore = useCallback(() => {
    const state = useSearchStore.getState();
    if (!state.loading && !state.loadingMore && state.hasMore) {
      void performSearch(false);
    }
  }, [performSearch]);

  return {
    query: store.query,
    setQuery: store.setQuery,
    author: store.author,
    setAuthor: store.setAuthor,
    items: store.items,
    totalFound: store.totalFound,
    loading: store.loading,
    loadingMore: store.loadingMore,
    error: store.error,
    hasSearched: store.hasSearched,
    hasMore: store.hasMore,
    scrollTop: store.scrollTop,
    setScrollTop: store.setScrollTop,
    filters: store.filters,
    setFilters: store.setFilters,
    loadMore,
    retry: () => void performSearch(true),
  };
}
