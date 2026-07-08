// Страница поиска: строка запроса (lazy-search), клиентские фильтры в правом
// сайдбаре, виртуализированный список результатов с догрузкой.

import { useVirtualizer } from "@tanstack/react-virtual";
import { Search as SearchIcon, SlidersHorizontal, TriangleAlert } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { FiltersSidebar } from "@/components/FiltersSidebar";
import { BackButton } from "@/components/PageHeader";
import { ResultRow } from "@/components/ResultRow";
import { Button, EmptyState, Input, Spinner } from "@/components/ui";
import { useSearch } from "@/hooks/useSearch";
import { applyFilters, hasActiveFilters } from "@/lib/filters";

export function SearchPage() {
  const search = useSearch();
  const filters = search.filters;
  const setFilters = search.setFilters;
  const [showFilters, setShowFilters] = useState(true);

  const visible = useMemo(() => applyFilters(search.items, filters), [search.items, filters]);

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

  // Догрузка при приближении к концу списка. Работает и при активных
  // клиентских фильтрах: подтягиваем следующие страницы до серверного предела.
  const virtualItems = virtualizer.getVirtualItems();
  useEffect(() => {
    if (!search.hasMore) return;
    const last = virtualItems.at(-1);
    const nearEnd = last ? last.index >= visible.length - 5 : visible.length === 0;
    if (nearEnd) search.loadMore();
  }, [virtualItems, visible.length, search]);

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
              placeholder="Поиск раздач на rutracker…"
              className="pl-9"
              autoFocus
            />
          </div>
          <Button
            variant={showFilters || hasActiveFilters(filters) ? "primary" : "secondary"}
            onClick={() => setShowFilters((v) => !v)}
          >
            <SlidersHorizontal className="h-4 w-4" />
            Фильтры
          </Button>
        </div>

        {search.hasSearched && !search.loading && (
          <div className="flex items-center gap-2 text-xs text-faint">
            <span>
              Показано {visible.length} из {search.totalFound}
              {search.totalFound >= 500 ? "+" : ""} найденных
            </span>
            {search.loadingMore && <Spinner className="h-3 w-3" />}
          </div>
        )}
      </header>

      <div className="flex min-h-0 flex-1">
        <div ref={scrollRef} className="min-w-0 flex-1 overflow-y-auto">
          {search.loading ? (
            <div className="flex h-full items-center justify-center">
              <Spinner className="h-6 w-6" />
            </div>
          ) : search.error ? (
            <SearchError message={search.error.message} onRetry={search.retry} />
          ) : !search.hasSearched ? (
            <EmptyState
              icon={<SearchIcon className="h-10 w-10" />}
              title="Начните поиск"
              hint="Введите название фильма, игры, дистрибутива или программы. Результаты можно мгновенно уточнять фильтрами справа без повторного запроса."
            />
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
                  <ResultRow result={visible[row.index]} />
                </div>
              ))}
            </div>
          )}
        </div>

        {showFilters && <FiltersSidebar filters={filters} onChange={setFilters} />}
      </div>
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
