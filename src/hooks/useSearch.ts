// Lazy-search: дебаунс запроса, отмена устаревших ответов, догрузка страниц.

import { useCallback, useEffect, useRef, useState } from "react";

import { api, ApiError } from "@/lib/api";
import type { SearchResult } from "@/lib/types";

const DEBOUNCE_MS = 450;

export interface SearchState {
  query: string;
  author: string;
  items: SearchResult[];
  totalFound: number;
  loading: boolean;
  loadingMore: boolean;
  error: ApiError | null;
  hasSearched: boolean;
  hasMore: boolean;
}

export function useSearch() {
  const [query, setQuery] = useState("");
  const [author, setAuthor] = useState("");
  const [state, setState] = useState<Omit<SearchState, "query" | "author">>({
    items: [],
    totalFound: 0,
    loading: false,
    loadingMore: false,
    error: null,
    hasSearched: false,
    hasMore: false,
  });

  // Идентификатор актуального запроса: ответы с меньшим id отбрасываются.
  const runId = useRef(0);
  const searchId = useRef<string | null>(null);

  const performSearch = useCallback(
    async (reset: boolean) => {
      const currentQuery = query.trim();
      const currentAuthor = author.trim();
      if (!currentQuery && !currentAuthor) return;

      const id = ++runId.current;
      setState((prev) => ({
        ...prev,
        loading: reset,
        loadingMore: !reset,
        error: null,
      }));

      try {
        const offset = reset ? 0 : state.items.length;
        const page = await api.search({
          query: currentQuery,
          forums: [],
          author: currentAuthor || null,
          sort: "seeders",
          order: "desc",
          offset,
          search_id: reset ? null : searchId.current,
        });

        if (id !== runId.current) return; // устаревший ответ

        searchId.current = page.search_id;
        setState((prev) => {
          const items = reset ? page.items : [...prev.items, ...page.items];
          return {
            items,
            totalFound: page.total_found,
            loading: false,
            loadingMore: false,
            error: null,
            hasSearched: true,
            hasMore: items.length < page.total_found && page.items.length > 0,
          };
        });
      } catch (error) {
        if (id !== runId.current) return;
        setState((prev) => ({
          ...prev,
          loading: false,
          loadingMore: false,
          error: error instanceof ApiError ? error : new ApiError("error", String(error)),
        }));
      }
    },
    [query, author, state.items.length],
  );

  // Дебаунс: новый поиск через паузу после последнего изменения строки.
  useEffect(() => {
    if (!query.trim() && !author.trim()) {
      runId.current++;
      searchId.current = null;
      setState({
        items: [],
        totalFound: 0,
        loading: false,
        loadingMore: false,
        error: null,
        hasSearched: false,
        hasMore: false,
      });
      return;
    }
    const timer = setTimeout(() => void performSearch(true), DEBOUNCE_MS);
    return () => clearTimeout(timer);
    // performSearch намеренно исключён: реагируем только на изменение запроса.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, author]);

  const loadMore = useCallback(() => {
    if (!state.loading && !state.loadingMore && state.hasMore) {
      void performSearch(false);
    }
  }, [state.loading, state.loadingMore, state.hasMore, performSearch]);

  return {
    query,
    setQuery,
    author,
    setAuthor,
    ...state,
    loadMore,
    retry: () => void performSearch(true),
  };
}
