// Страница поиска: строка запроса (lazy-search), клиентские фильтры,
// виртуализированный список результатов с догрузкой.

import { useVirtualizer } from "@tanstack/react-virtual";
import { Search as SearchIcon, SlidersHorizontal, TriangleAlert } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { FiltersPanel } from "@/components/FiltersPanel";
import { ResultRow } from "@/components/ResultRow";
import { Button, EmptyState, Input, Spinner } from "@/components/ui";
import { useSearch } from "@/hooks/useSearch";
import { applyFilters, DEFAULT_FILTERS, hasActiveFilters, type ClientFilters } from "@/lib/filters";

export function SearchPage() {
  const search = useSearch();
  const [filters, setFilters] = useState<ClientFilters>(DEFAULT_FILTERS);
  const [showFilters, setShowFilters] = useState(false);

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

  // Догрузка при приближении к концу списка.
  const virtualItems = virtualizer.getVirtualItems();
  useEffect(() => {
    const last = virtualItems.at(-1);
    if (!last) return;
    if (last.index >= visible.length - 5 && search.hasMore && filters.refine.trim() === "") {
      search.loadMore();
    }
  }, [virtualItems, visible.length, search, filters.refine]);

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-col gap-3 border-b border-border px-5 py-4">
        <div className="flex items-center gap-2">
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

        {showFilters && <FiltersPanel filters={filters} onChange={setFilters} />}

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

      <div ref={scrollRef} className="flex-1 overflow-y-auto">
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
            hint="Введите название фильма, игры, дистрибутива или программы. Результаты можно мгновенно уточнять фильтрами без повторного запроса."
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
