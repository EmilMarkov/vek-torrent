// Страница отслеживаемого: фильтры, пагинация, детект обновлений с деталями
// и переходом к истории изменений.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BellOff,
  BellRing,
  Eye,
  FileClock,
  RefreshCw,
  Search as SearchIcon,
  Trash2,
} from "lucide-react";
import { useMemo, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";

import { PageHeader } from "@/components/PageHeader";
import { toast } from "@/components/Toaster";
import { Badge, Button, EmptyState, Input, Pagination, Spinner, Toggle } from "@/components/ui";
import { useFavorites } from "@/hooks/useLibrary";
import { api } from "@/lib/api";
import { formatDate } from "@/lib/format";
import type { FavoriteItem } from "@/lib/types";
import { useAppStore } from "@/store";

const PAGE_SIZE = 20;

export function FavoritesPage() {
  const queryClient = useQueryClient();
  const openTopic = useAppStore((s) => s.openTopic);
  const navigate = useAppStore((s) => s.navigate);
  const { data, isLoading } = useFavorites();
  const { data: config } = useQuery({ queryKey: ["config"], queryFn: api.getConfig });
  const [checking, setChecking] = useState(false);

  // Фильтры и пагинация — клиентские (данные локальные).
  const [query, setQuery] = useState("");
  const [onlyUpdated, setOnlyUpdated] = useState(false);
  const [page, setPage] = useState(1);

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["favorites"] });

  const check = async () => {
    setChecking(true);
    try {
      const updated = await api.checkFavorites();
      queryClient.setQueryData(["favorites"], updated);
      const count = updated.filter((f) => f.hasUpdate).length;
      toast.success(count > 0 ? `Обновлений найдено: ${count}` : "Обновлений нет");
    } catch {
      toast.error("Не удалось проверить обновления");
    } finally {
      setChecking(false);
    }
  };

  const open = async (fav: FavoriteItem) => {
    // По умолчанию метка снимается при открытии; в настройках можно
    // переключить на ручной сброс (кнопка-колокольчик). Пока конфиг не
    // загружен, метку не трогаем — сбросить вручную всегда можно.
    if (fav.hasUpdate && config?.favorites.auto_clear_update === true) {
      await api.clearFavoriteUpdate(fav.topicId);
      refresh();
    }
    openTopic(fav.topicId);
  };

  const clearUpdate = async (topicId: number) => {
    await api.clearFavoriteUpdate(topicId);
    refresh();
  };

  const remove = async (fav: FavoriteItem) => {
    // Снятие с отслеживания стирает историю изменений и версии файлов —
    // обязательно предупреждаем.
    if (fav.historyCount > 0) {
      const confirmed = await ask(
        `Вместе с раздачей будет безвозвратно удалена история изменений (событий: ${fav.historyCount}) и сохранённые версии файлов — скачать патч для неё будет нельзя.`,
        { title: "Перестать отслеживать?", kind: "warning" },
      );
      if (!confirmed) return;
    }
    await api.removeFavorite(fav.topicId);
    refresh();
  };

  const favorites = useMemo(() => data ?? [], [data]);
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return favorites.filter((f) => {
      if (q && !f.title.toLowerCase().includes(q)) return false;
      if (onlyUpdated && !f.hasUpdate) return false;
      return true;
    });
  }, [favorites, query, onlyUpdated]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount);
  const visible = filtered.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Отслеживаемое"
        actions={
          favorites.length > 0 && (
            <Button variant="secondary" loading={checking} onClick={check}>
              <RefreshCw className="h-4 w-4" />
              Проверить обновления
            </Button>
          )
        }
      />

      {favorites.length > 0 && (
        <div className="flex items-center gap-4 border-b border-border px-5 py-3">
          <div className="relative max-w-xs flex-1">
            <SearchIcon className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-faint" />
            <Input
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setPage(1);
              }}
              placeholder="Фильтр по названию…"
              className="pl-9"
            />
          </div>
          <Toggle
            checked={onlyUpdated}
            onChange={(v) => {
              setOnlyUpdated(v);
              setPage(1);
            }}
            label="Только с обновлениями"
          />
          <span className="ml-auto text-xs text-faint">
            {filtered.length} из {favorites.length}
          </span>
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex h-full items-center justify-center">
            <Spinner className="h-6 w-6" />
          </div>
        ) : favorites.length === 0 ? (
          <EmptyState
            icon={<Eye className="h-10 w-10" />}
            title="Пока ничего не отслеживается"
            hint="Открывайте страницы раздач и добавляйте их в отслеживаемое — приложение будет следить за их обновлениями на трекере и вести историю изменений."
          />
        ) : visible.length === 0 ? (
          <EmptyState
            icon={<SearchIcon className="h-10 w-10" />}
            title="Ничего не найдено"
            hint="Попробуйте изменить фильтры."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border/60 px-3">
            {visible.map((fav) => (
              <div key={fav.topicId} className="group flex items-start gap-3 py-2.5">
                <button onClick={() => void open(fav)} className="min-w-0 flex-1 text-left">
                  <div className="flex items-center gap-2">
                    {fav.hasUpdate && (
                      <Badge tone="accent">
                        <BellRing className="mr-1 h-3 w-3" />
                        обновление
                      </Badge>
                    )}
                    <span className="truncate text-sm font-medium text-text group-hover:text-accent">
                      {fav.title}
                    </span>
                  </div>
                  <div className="mt-0.5 text-xs text-faint">
                    В избранном с {formatDate(fav.addedAt)} · проверено{" "}
                    {formatDate(fav.lastChecked)}
                  </div>
                  {/* Что именно изменилось (если детали известны). */}
                  {fav.hasUpdate && fav.changes.length > 0 && (
                    <ul className="mt-1 flex flex-col gap-0.5 text-xs text-warn">
                      {fav.changes.map((change, i) => (
                        <li key={i}>• {change}</li>
                      ))}
                    </ul>
                  )}
                </button>
                <div className="flex shrink-0 items-center gap-1">
                  {fav.hasUpdate && (
                    <Button
                      variant="ghost"
                      onClick={() => void clearUpdate(fav.topicId)}
                      title="Отметить просмотренным (снять метку обновления)"
                    >
                      <BellOff className="h-4 w-4" />
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    onClick={() =>
                      navigate({ kind: "tracked-history", topicId: fav.topicId, title: fav.title })
                    }
                    title={`История изменений (${fav.historyCount})`}
                  >
                    <FileClock className="h-4 w-4" />
                    {fav.historyCount > 0 && (
                      <span className="text-xs text-muted">{fav.historyCount}</span>
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => void remove(fav)}
                    title="Перестать отслеживать"
                    className="opacity-0 group-hover:opacity-100"
                  >
                    <Trash2 className="h-4 w-4 text-danger" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {pageCount > 1 && (
        <div className="border-t border-border px-4 py-2">
          <Pagination page={currentPage} pageCount={pageCount} onChange={setPage} />
        </div>
      )}
    </div>
  );
}
