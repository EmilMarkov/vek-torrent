// Страница избранного: список, детект обновлений, ручная проверка.

import { useQueryClient } from "@tanstack/react-query";
import { BellRing, Heart, RefreshCw, Trash2 } from "lucide-react";
import { useState } from "react";

import { PageHeader } from "@/components/PageHeader";
import { toast } from "@/components/Toaster";
import { Badge, Button, EmptyState, Spinner } from "@/components/ui";
import { useFavorites } from "@/hooks/useLibrary";
import { api } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { useAppStore } from "@/store";

export function FavoritesPage() {
  const queryClient = useQueryClient();
  const openTopic = useAppStore((s) => s.openTopic);
  const { data, isLoading } = useFavorites();
  const [checking, setChecking] = useState(false);

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

  const open = async (topicId: number, hasUpdate: boolean) => {
    if (hasUpdate) {
      await api.clearFavoriteUpdate(topicId);
      refresh();
    }
    openTopic(topicId);
  };

  const remove = async (topicId: number) => {
    await api.removeFavorite(topicId);
    refresh();
  };

  const favorites = data ?? [];

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Избранное"
        actions={
          favorites.length > 0 && (
            <Button variant="secondary" loading={checking} onClick={check}>
              <RefreshCw className="h-4 w-4" />
              Проверить обновления
            </Button>
          )
        }
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex h-full items-center justify-center">
            <Spinner className="h-6 w-6" />
          </div>
        ) : favorites.length === 0 ? (
          <EmptyState
            icon={<Heart className="h-10 w-10" />}
            title="В избранном пока пусто"
            hint="Открывайте страницы раздач и добавляйте их в избранное — приложение будет следить за их обновлениями на трекере."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border/60 px-3">
            {favorites.map((fav) => (
              <div key={fav.topicId} className="group flex items-center gap-3 py-2.5">
                <button
                  onClick={() => open(fav.topicId, fav.hasUpdate)}
                  className="min-w-0 flex-1 text-left"
                >
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
                </button>
                <Button
                  variant="ghost"
                  onClick={() => remove(fav.topicId)}
                  title="Убрать из избранного"
                  className="shrink-0 opacity-0 group-hover:opacity-100"
                >
                  <Trash2 className="h-4 w-4 text-danger" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
