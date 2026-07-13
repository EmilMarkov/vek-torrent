// Lazy-search: дебаунс запроса, отмена устаревших ответов.
//
// Все серверные страницы результатов (rutracker отдаёт по 50, максимум 500)
// загружаются в буфер, а пагинация в UI — КЛИЕНТСКАЯ поверх ОТФИЛЬТРОВАННОГО
// набора. Так клиентские фильтры (уточнение/размер/сиды/проверенные) видят все
// найденные раздачи, а не только текущую страницу, и «страница X из Y»
// считается по числу подходящих результатов.
//
// Серверные фильтры (автор, разделы, категории) уходят в запрос rutracker
// (`pn=`, `f[]=`), поэтому влияют на само число найденного.
//
// Состояние хранится в zustand-сторе (а не в локальном useState страницы),
// поэтому переживает уход со страницы «Поиск» и возврат по кнопке «Назад».

import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { create } from "zustand";

import { useForumGroups } from "@/components/ForumTreePicker";
import { api, ApiError } from "@/lib/api";
import { DEFAULT_FILTERS, effectiveCategoryForumIds, type ClientFilters } from "@/lib/filters";
import { pushSearchHistory } from "@/lib/searchHistory";
import type { SearchResult, SortField, SortOrder } from "@/lib/types";

const DEBOUNCE_MS = 450;

/** Размер страницы в интерфейсе (пагинация отфильтрованного набора). */
export const SEARCH_PAGE_SIZE = 50;
/** Максимум результатов, который отдаёт rutracker. */
const SERVER_CAP = 500;

interface SearchStore {
  query: string;
  /** Серверная сортировка rutracker (клик по заголовку таблицы). */
  sort: SortField;
  order: SortOrder;
  /** Все загруженные результаты поиска (по всем серверным страницам). */
  items: SearchResult[];
  totalFound: number;
  /** Текущая страница интерфейса (с 1) — поверх отфильтрованного набора. */
  displayPage: number;
  /** Идёт первичная загрузка (первая страница). */
  loading: boolean;
  /** Догружаются остальные серверные страницы в фоне. */
  loadingMore: boolean;
  error: ApiError | null;
  hasSearched: boolean;
  /** Ключ (запрос+автор+разделы) последнего успешного поиска — защита от
   *  повторного запуска того же поиска при возврате на страницу. */
  lastSearchedKey: string;
  /** Сохранённая позиция прокрутки списка результатов. */
  scrollTop: number;
  /** Фильтры (сохраняются между переходами). */
  filters: ClientFilters;

  setQuery: (query: string) => void;
  setDisplayPage: (page: number) => void;
  setScrollTop: (scrollTop: number) => void;
  setFilters: (filters: ClientFilters) => void;
}

// Служебные изменяемые значения вне рендера: идентификатор актуального
// запроса (ответы с меньшим id отбрасываются) и таймер дебаунса.
let runId = 0;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;

const useSearchStore = create<SearchStore>((set) => ({
  query: "",
  sort: "seeders",
  order: "desc",
  items: [],
  totalFound: 0,
  displayPage: 1,
  loading: false,
  loadingMore: false,
  error: null,
  hasSearched: false,
  lastSearchedKey: "",
  scrollTop: 0,
  filters: DEFAULT_FILTERS,
  setQuery: (query) => set({ query }),
  setDisplayPage: (displayPage) => set({ displayPage }),
  setScrollTop: (scrollTop) => set({ scrollTop }),
  setFilters: (filters) => set({ filters }),
}));

export function useSearch() {
  const store = useSearchStore();

  // Данные для разворачивания категорий в наборы разделов rutracker.
  const forumGroups = useForumGroups();
  const { data: userCategories } = useQuery({
    queryKey: ["user-categories"],
    queryFn: api.userCategories,
  });

  // Серверный набор разделов: выбранные категории + ручной выбор в дереве.
  const serverForums = useMemo(() => {
    const ids = new Set<number>(store.filters.forumIds);
    for (const categoryId of store.filters.categoryIds) {
      const category = (userCategories ?? []).find((c) => c.id === categoryId);
      if (!category) continue;
      for (const id of effectiveCategoryForumIds(forumGroups.data ?? [], category)) {
        ids.add(id);
      }
    }
    return [...ids].sort((a, b) => a - b);
  }, [store.filters.forumIds, store.filters.categoryIds, userCategories, forumGroups.data]);

  // Актуальный набор разделов для стабильного runSearch (обновляем в эффекте,
  // а не переприсваиванием модульной переменной в рендере).
  const forumsRef = useRef(serverForums);
  useEffect(() => {
    forumsRef.current = serverForums;
  }, [serverForums]);

  // Готовность серверных фильтров: пока категории не разрешились в разделы,
  // поиск с ними уходить не должен (иначе выдача — без фильтра категории).
  const filtersReady = useMemo(() => {
    if (store.filters.categoryIds.length === 0) return true;
    if (userCategories === undefined) return false;
    const needsTree = store.filters.categoryIds.some((id) => {
      const c = userCategories.find((x) => x.id === id);
      return c !== undefined && c.forumIds.length === 0;
    });
    return !needsTree || forumGroups.data !== undefined || forumGroups.isError;
  }, [store.filters.categoryIds, userCategories, forumGroups.data, forumGroups.isError]);

  // Отсев «призрачных» категорий: id удалённой категории не должен висеть в
  // фильтрах (иначе фильтры выглядят активными без видимого выбора).
  useEffect(() => {
    if (!userCategories) return;
    const known = new Set(userCategories.map((c) => c.id));
    const current = useSearchStore.getState().filters;
    const pruned = current.categoryIds.filter((id) => known.has(id));
    if (pruned.length !== current.categoryIds.length) {
      useSearchStore.setState({ filters: { ...current, categoryIds: pruned } });
    }
  }, [userCategories]);

  /**
   * Запускает новый поиск: грузит первую серверную страницу, затем в фоне —
   * остальные, накапливая их в буфер (до серверного предела). Устаревшие
   * запросы отбрасываются по `runId`.
   */
  const runSearch = useCallback(async function run(): Promise<void> {
    const state = useSearchStore.getState();
    const currentQuery = state.query.trim();
    const currentAuthor = state.filters.author.trim();
    if (!currentQuery && !currentAuthor) return;

    const forums = forumsRef.current;
    const id = ++runId;
    useSearchStore.setState({
      loading: true,
      loadingMore: false,
      error: null,
      items: [],
      displayPage: 1,
    });

    const request = (offset: number, searchId: string | null) =>
      api.search({
        query: currentQuery,
        forums,
        author: currentAuthor || null,
        sort: state.sort,
        order: state.order,
        offset,
        search_id: searchId,
      });

    try {
      const first = await request(0, null);
      if (id !== runId) return; // устаревший ответ

      if (currentQuery) pushSearchHistory(currentQuery);
      let items = first.items;
      useSearchStore.setState({
        items,
        totalFound: first.total_found,
        loading: false,
        loadingMore: items.length < Math.min(first.total_found, SERVER_CAP),
        error: null,
        hasSearched: true,
        lastSearchedKey: searchKey(currentQuery, currentAuthor, forums),
        scrollTop: 0,
      });

      // Догрузка остальных страниц в фоне (одна серверная сессия).
      let searchId = first.search_id;
      while (items.length < Math.min(first.total_found, SERVER_CAP)) {
        const next = await request(items.length, searchId);
        if (id !== runId) return;
        if (next.items.length === 0) break;
        searchId = next.search_id ?? searchId;
        items = items.concat(next.items);
        useSearchStore.setState({
          items,
          totalFound: next.total_found,
          loadingMore: items.length < Math.min(next.total_found, SERVER_CAP),
        });
      }
      if (id === runId) useSearchStore.setState({ loadingMore: false });
    } catch (error) {
      if (id !== runId) return;
      useSearchStore.setState({
        loading: false,
        loadingMore: false,
        items: [],
        totalFound: 0,
        error: error instanceof ApiError ? error : new ApiError("error", String(error)),
      });
    }
  }, []);

  // Дебаунс: новый поиск через паузу после изменения запроса или серверных
  // фильтров (автор, разделы, категории).
  const author = store.filters.author.trim();
  const forumsKey = serverForums.join(",");
  useEffect(() => {
    if (!store.query.trim() && !author) {
      runId++;
      useSearchStore.setState({
        items: [],
        totalFound: 0,
        displayPage: 1,
        loading: false,
        loadingMore: false,
        error: null,
        hasSearched: false,
        lastSearchedKey: "",
        scrollTop: 0,
      });
      return;
    }
    // Не запускаем поиск, пока категории не разрешились в разделы.
    if (!filtersReady) return;
    // Возврат на страницу с неизменными параметрами: результаты уже в сторе.
    const key = searchKey(store.query.trim(), author, serverForums);
    if (key === useSearchStore.getState().lastSearchedKey) return;
    debounceTimer = setTimeout(() => void runSearch(), DEBOUNCE_MS);
    return () => clearTimeout(debounceTimer);
  }, [store.query, author, forumsKey, filtersReady, serverForums, runSearch]);

  // Смена сортировки — новый поиск сразу, без дебаунса.
  const setSort = useCallback(
    (sort: SortField, order: SortOrder) => {
      clearTimeout(debounceTimer);
      useSearchStore.setState({ sort, order });
      void runSearch();
    },
    [runSearch],
  );

  return {
    query: store.query,
    setQuery: store.setQuery,
    sort: store.sort,
    order: store.order,
    setSort,
    items: store.items,
    totalFound: store.totalFound,
    displayPage: store.displayPage,
    setDisplayPage: store.setDisplayPage,
    loading: store.loading,
    loadingMore: store.loadingMore,
    error: store.error,
    hasSearched: store.hasSearched,
    scrollTop: store.scrollTop,
    setScrollTop: store.setScrollTop,
    filters: store.filters,
    setFilters: store.setFilters,
    retry: () => void runSearch(),
  };
}

/** Ключ серверных параметров поиска (для дедупликации перезапусков). */
function searchKey(query: string, author: string, forums: number[]): string {
  return `${query}\n${author}\n${forums.join(",")}`;
}
