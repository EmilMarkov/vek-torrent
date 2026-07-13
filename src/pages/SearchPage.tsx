// Страница поиска: строка запроса (lazy-search), клиентские фильтры в правом
// сайдбаре, таблица результатов с серверной сортировкой и пагинацией.

import { useVirtualizer } from "@tanstack/react-virtual";
import {
  ArrowDown,
  ArrowUp,
  Search as SearchIcon,
  SlidersHorizontal,
  TriangleAlert,
} from "lucide-react";
import { clsx } from "clsx";
import { useEffect, useMemo, useRef, useState } from "react";

import { FiltersSidebar } from "@/components/FiltersSidebar";
import { BackButton } from "@/components/PageHeader";
import { SearchHistoryDropdown } from "@/components/SearchHistory";
import { RESULT_COLS, ResultRow } from "@/components/ResultRow";
import { Button, EmptyState, Input, Pagination, Spinner } from "@/components/ui";
import { SEARCH_PAGE_SIZE, useSearch } from "@/hooks/useSearch";
import {
  applyFilters,
  hasActiveFilters,
  hasClientFilters,
  type ClientFilters,
} from "@/lib/filters";
import type { SortField, SortOrder } from "@/lib/types";

export function SearchPage() {
  const search = useSearch();
  const filters = search.filters;
  const setFilters = search.setFilters;
  const [showFilters, setShowFilters] = useState(true);
  const [historyOpen, setHistoryOpen] = useState(false);

  // Отфильтрованный набор (клиентские фильтры поверх всех загруженных
  // результатов) и клиентская пагинация поверх него.
  const filtered = useMemo(() => applyFilters(search.items, filters), [search.items, filters]);
  const pageCount = Math.max(1, Math.ceil(filtered.length / SEARCH_PAGE_SIZE));
  const displayPage = Math.min(search.displayPage, pageCount);
  const visible = useMemo(
    () => filtered.slice((displayPage - 1) * SEARCH_PAGE_SIZE, displayPage * SEARCH_PAGE_SIZE),
    [filtered, displayPage],
  );

  const scrollRef = useRef<HTMLDivElement>(null);
  // TanStack Virtual возвращает нестабильные функции — это ожидаемо здесь.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 68,
    overscan: 8,
  });

  // Восстанавливаем позицию прокрутки при возврате на страницу и сохраняем её
  // при уходе (чтобы «Назад» возвращал ровно туда, где были).
  const setScrollTop = search.setScrollTop;
  const savedScrollTop = search.scrollTop;
  useEffect(() => {
    const el = scrollRef.current;
    if (el && savedScrollTop > 0) el.scrollTop = savedScrollTop;
    // Тот же DOM-узел живёт весь срок страницы: читаем его scrollTop при уходе.
    return () => {
      if (el) setScrollTop(el.scrollTop);
    };
    // Только при монтировании/размонтировании страницы.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const virtualItems = virtualizer.getVirtualItems();

  // Переход по страницам (клиентский): скроллим список к началу.
  const goToPage = (page: number) => {
    search.setDisplayPage(page);
    scrollRef.current?.scrollTo({ top: 0 });
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-col gap-3 border-b border-border px-5 py-4">
        <div className="flex items-center gap-2">
          <BackButton />
          <div className="relative flex-1">
            <SearchIcon className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-faint" />
            <Input
              value={search.query}
              onChange={(e) => search.setQuery(e.target.value)}
              onFocus={() => setHistoryOpen(true)}
              onBlur={() => setHistoryOpen(false)}
              onKeyDown={(e) => e.key === "Escape" && setHistoryOpen(false)}
              placeholder="Поиск раздач на rutracker…"
              className="pl-9"
              autoFocus
            />
            {historyOpen && (
              <SearchHistoryDropdown
                query={search.query}
                onPick={search.setQuery}
                onClose={() => setHistoryOpen(false)}
              />
            )}
          </div>
          <Button
            variant={showFilters || hasActiveFilters(filters) ? "primary" : "secondary"}
            onClick={() => setShowFilters((v) => !v)}
          >
            <SlidersHorizontal className="h-4 w-4" />
            Фильтры
          </Button>
        </div>

        {search.hasSearched && !search.loading && !search.error && (
          <div className="flex items-center gap-2 text-xs text-faint">
            <span>
              {hasClientFilters(filters)
                ? `Подходит ${filtered.length} из ${search.totalFound}`
                : `Найдено ${search.totalFound}`}
              {search.totalFound >= 500 ? "+" : ""} · страница {displayPage} из {pageCount}
            </span>
            {search.loadingMore && (
              <span className="flex items-center gap-1">
                <Spinner className="h-3 w-3" />
                догрузка…
              </span>
            )}
          </div>
        )}
      </header>

      <div className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
            {search.error ? (
              <SearchError message={search.error.message} onRetry={search.retry} />
            ) : !search.hasSearched && !search.loading ? (
              <EmptyState
                icon={<SearchIcon className="h-10 w-10" />}
                title="Начните поиск"
                hint="Введите название фильма, игры, дистрибутива или программы. Результаты можно мгновенно уточнять фильтрами справа без повторного запроса."
              />
            ) : (
              // Шапка не размонтируется во время загрузки и при пустой
              // фильтрации: сортировку можно менять в любой момент.
              <>
                <TableHeader sort={search.sort} order={search.order} onSort={search.setSort} />
                {search.loading ? (
                  <div className="flex justify-center py-16">
                    <Spinner className="h-6 w-6" />
                  </div>
                ) : visible.length === 0 ? (
                  <EmptyState
                    icon={<SearchIcon className="h-10 w-10" />}
                    title="Ничего не найдено"
                    hint={
                      hasActiveFilters(filters)
                        ? "Попробуйте ослабить фильтры."
                        : "Попробуйте изменить запрос."
                    }
                  />
                ) : (
                  <div className="relative" style={{ height: `${virtualizer.getTotalSize()}px` }}>
                    {virtualItems.map((row) => (
                      <div
                        key={visible[row.index].topic_id}
                        className="absolute top-0 left-0 w-full px-3"
                        style={{ height: `${row.size}px`, transform: `translateY(${row.start}px)` }}
                      >
                        <ResultRow
                          result={visible[row.index]}
                          onPatchFilters={(patch: Partial<ClientFilters>) =>
                            setFilters({ ...filters, ...patch })
                          }
                        />
                      </div>
                    ))}
                  </div>
                )}
              </>
            )}
          </div>

          {search.hasSearched && pageCount > 1 && (
            <div className="border-t border-border px-4 py-2">
              <Pagination
                page={displayPage}
                pageCount={pageCount}
                onChange={goToPage}
                disabled={search.loading}
              />
            </div>
          )}
        </div>

        {showFilters && <FiltersSidebar filters={filters} onChange={setFilters} />}
      </div>
    </div>
  );
}

/** Порядок по умолчанию при первом клике на столбец. */
function defaultOrder(field: SortField): SortOrder {
  return field === "title" ? "asc" : "desc";
}

/** Шапка таблицы результатов: клик по столбцу — серверная сортировка. */
function TableHeader({
  sort,
  order,
  onSort,
}: {
  sort: SortField;
  order: SortOrder;
  onSort: (sort: SortField, order: SortOrder) => void;
}) {
  const cell = (label: string, field: SortField, className?: string) => {
    const active = sort === field;
    const toggle = () =>
      onSort(field, active ? (order === "desc" ? "asc" : "desc") : defaultOrder(field));
    return (
      <button
        onClick={toggle}
        className={clsx(
          "flex shrink-0 items-center gap-0.5 hover:text-text",
          active ? "text-text" : "text-faint",
          className,
        )}
        title="Сортировать (повторный клик меняет направление)"
      >
        <span className="truncate">{label}</span>
        {active &&
          (order === "desc" ? (
            <ArrowDown className="h-3 w-3 shrink-0" />
          ) : (
            <ArrowUp className="h-3 w-3 shrink-0" />
          ))}
      </button>
    );
  };

  return (
    <div className="sticky top-0 z-10 flex items-center gap-3 border-b border-border bg-bg px-3 py-2 text-[11px] font-medium">
      <div className="min-w-0 flex-1">{cell("Название", "title")}</div>
      {cell("Размер", "size", clsx(RESULT_COLS.size, "justify-end"))}
      {cell("Сиды", "seeders", clsx(RESULT_COLS.seeders, "justify-end"))}
      {cell("Личи", "leechers", clsx(RESULT_COLS.leechers, "justify-end"))}
      {cell("Скачали", "downloads", clsx(RESULT_COLS.downloads, "justify-end"))}
      {cell("Добавлена", "registered", clsx(RESULT_COLS.date, "justify-end"))}
      <div className={clsx(RESULT_COLS.actions, "shrink-0")} />
    </div>
  );
}

function SearchError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <EmptyState
      icon={<TriangleAlert className="h-10 w-10 text-warn" />}
      title="Не удалось выполнить поиск"
      hint={
        <span className="flex flex-col items-center gap-3">
          {message}
          <Button variant="secondary" onClick={onRetry}>
            Повторить
          </Button>
        </span>
      }
    />
  );
}
