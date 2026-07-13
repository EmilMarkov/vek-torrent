// Lazy-search с серверной пагинацией: дебаунс запроса, отмена устаревших
// ответов, постраничная навигация (rutracker отдаёт по 50, максимум 500).
//
// Фильтры по автору, разделам и категориям — СЕРВЕРНЫЕ (`pn=`, `f[]=`):
// их изменение перезапускает поиск, поэтому счётчик результатов и пагинация
// честные. Клиентскими остаются только уточнение/размер/сиды/проверенные —
// они действуют в пределах загруженной страницы.
//
// Состояние хранится в zustand-сторе (а не в локальном useState страницы),
// поэтому переживает уход со страницы «Поиск» и возврат по кнопке «Назад» —
// запрос, результаты и позиция прокрутки сохраняются.

import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo } from "react";
import { create } from "zustand";

import { useForumGroups } from "@/components/ForumTreePicker";
import { api, ApiError } from "@/lib/api";
import { DEFAULT_FILTERS, effectiveCategoryForumIds, type ClientFilters } from "@/lib/filters";
import { pushSearchHistory } from "@/lib/searchHistory";
import type { SearchResult, SortField, SortOrder } from "@/lib/types";

const DEBOUNCE_MS = 450;

/** Размер серверной страницы rutracker. */
export const SEARCH_PAGE_SIZE = 50;
/** Максимум результатов, который отдаёт rutracker. */
const SERVER_CAP = 500;

interface SearchStore {
  query: string;
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
  /** Ключ (запрос+серверные фильтры) последнего успешного поиска: защита от
   *  повторного запуска того же поиска при возврате на страницу. */
  lastSearchedKey: string;
  /** Сохранённая позиция прокрутки списка результатов. */
  scrollTop: number;
  /** Фильтры (сохраняются между переходами). */
  filters: ClientFilters;

  setQuery: (query: string) => void;
  setScrollTop: (scrollTop: number) => void;
  setFilters: (filters: ClientFilters) => void;
}

// Служебные изменяемые значения вне рендера: токен продолжения, идентификатор
// актуального запроса (ответы с меньшим id отбрасываются — «отмена устаревших»)
// и таймер дебаунса (сбрасывается при немедленном поиске из setSort).
let runId = 0;
let searchId: string | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | undefined;
// Актуальный серверный набор разделов (категории + дерево) — читается
// fetchPage в момент запроса.
let resolvedForums: number[] = [];
// Ключ параметров серверной сессии (запрос+автор+разделы), с которыми она
// РЕАЛЬНО создана. Стр. пагинации внутри сессии сохраняют этот ключ, а не
// текущее (возможно, уже изменившееся) состояние разделов.
let sessionKey = "";

const useSearchStore = create<SearchStore>((set) => ({
  query: "",
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
  resolvedForums = serverForums;

  // Готовность серверных фильтров: пока категории не разрешены в разделы,
  // поиск с ними уходить не должен (иначе выдача — без фильтра категории).
  // Категория с явными forum_ids дерева не требует; эвристическая — требует.
  const filtersReady = useMemo(() => {
    if (store.filters.categoryIds.length === 0) return true;
    if (userCategories === undefined) return false; // список категорий грузится
    const needsTree = store.filters.categoryIds.some((id) => {
      const c = userCategories.find((x) => x.id === id);
      return c !== undefined && c.forumIds.length === 0;
    });
    // Дерево нужно только эвристическим категориям; при ошибке загрузки —
    // работаем best-effort с тем, что есть.
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
   * Загружает страницу результатов. `newSession` начинает новую серверную
   * сессию поиска (rutracker фиксирует запрос, фильтры и порядок в
   * `search_id`, поэтому их смена требует сброса токена).
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
    const currentAuthor = state.filters.author.trim();
    if (!currentQuery && !currentAuthor) return;

    // Снимок разделов на момент запроса: и `forums`, и ключ сессии считаем от
    // одного значения (module-var может измениться между рендерами).
    const requestForums = resolvedForums;

    // Токен старой сессии не должен пережить смену запроса; ключ сессии
    // фиксируем параметрами, с которыми она реально создаётся.
    if (newSession) {
      searchId = null;
      sessionKey = searchKey(currentQuery, currentAuthor, requestForums);
    }

    const id = ++runId;
    useSearchStore.setState({ loading: true, error: null, page: targetPage });

    try {
      const page = await api.search({
        query: currentQuery,
        forums: requestForums,
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
        // Ключ сессии, а не текущих разделов: страница внутри сессии не должна
        // «запечатывать» ключ с параметрами, которых в этой сессии не было.
        lastSearchedKey: sessionKey,
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

  // Дебаунс: новый поиск через паузу после изменения запроса или серверных
  // фильтров (автор, разделы, категории).
  const author = store.filters.author.trim();
  const forumsKey = serverForums.join(",");
  useEffect(() => {
    if (!store.query.trim() && !author) {
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
    // Не запускаем поиск, пока категории не разрешились в разделы: иначе
    // выдача уйдёт без фильтра категории. Когда данные догрузятся, serverForums
    // изменится → эффект перезапустится.
    if (!filtersReady) return;
    // Возврат на страницу с неизменными параметрами: результаты, номер
    // страницы и позиция прокрутки уже в сторе — повторный поиск не нужен.
    const key = searchKey(store.query.trim(), author, serverForums);
    if (key === useSearchStore.getState().lastSearchedKey) return;
    debounceTimer = setTimeout(() => void fetchPage(1, true), DEBOUNCE_MS);
    return () => clearTimeout(debounceTimer);
  }, [store.query, author, forumsKey, filtersReady, serverForums, fetchPage]);

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

/** Ключ серверных параметров поиска (для дедупликации перезапусков). */
function searchKey(query: string, author: string, forums: number[]): string {
  return `${query}\n${author}\n${forums.join(",")}`;
}
