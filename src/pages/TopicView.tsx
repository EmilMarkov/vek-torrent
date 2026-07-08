// Страница раздачи: заголовок, статистика, действия и блочный рендер содержимого.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowDownToLine,
  ArrowLeft,
  ArrowUp,
  Heart,
  Magnet,
  TriangleAlert,
  Users,
} from "lucide-react";
import { clsx } from "clsx";
import { openUrl } from "@tauri-apps/plugin-opener";

import { ContentBlocks } from "@/components/ContentBlockView";
import { openDownloadModal } from "@/components/DownloadModal";
import { toast } from "@/components/Toaster";
import { Badge, Button, EmptyState, Spinner } from "@/components/ui";
import { useAddDownload } from "@/hooks/useAddDownload";
import { api } from "@/lib/api";
import { formatNumber, formatSize } from "@/lib/format";
import { useAppStore } from "@/store";

export function TopicView({ topicId }: { topicId: number }) {
  const back = useAppStore((s) => s.back);
  const { add, adding } = useAddDownload();
  const queryClient = useQueryClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ["topic", topicId],
    queryFn: () => api.topic(topicId),
    retry: false,
    // Кэшируем страницу раздачи на сессию: возврат по «Назад» не перезагружает.
    staleTime: Infinity,
    gcTime: Infinity,
  });

  const { data: favorite } = useQuery({
    queryKey: ["is-favorite", topicId],
    queryFn: () => api.isFavorite(topicId),
  });

  const toggleFavorite = async () => {
    try {
      if (favorite) await api.removeFavorite(topicId);
      else await api.addFavorite(topicId);
      await queryClient.invalidateQueries({ queryKey: ["is-favorite", topicId] });
      await queryClient.invalidateQueries({ queryKey: ["favorites"] });
      toast.success(favorite ? "Убрано из избранного" : "Добавлено в избранное");
    } catch {
      toast.error("Не удалось изменить избранное");
    }
  };

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-3 border-b border-border px-5 py-3">
        <Button variant="ghost" onClick={back} className="px-2">
          <ArrowLeft className="h-4 w-4" />
          Назад
        </Button>
        {data && (
          <nav className="flex items-center gap-1.5 truncate text-xs text-faint">
            {data.forum_path.map((f, i) => (
              <span key={f.id} className="truncate">
                {i > 0 && <span className="mr-1.5">/</span>}
                {f.name}
              </span>
            ))}
          </nav>
        )}
      </header>

      <div className="flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex h-full items-center justify-center">
            <Spinner className="h-6 w-6" />
          </div>
        ) : error || !data ? (
          <EmptyState
            icon={<TriangleAlert className="h-10 w-10 text-warn" />}
            title="Не удалось загрузить раздачу"
            hint={error instanceof Error ? error.message : undefined}
          />
        ) : (
          <div className="mx-auto max-w-4xl px-6 py-5">
            <h1 className="selectable text-xl leading-snug font-semibold text-text">
              {data.title}
            </h1>

            <div className="mt-3 flex flex-wrap items-center gap-2">
              <Button
                variant="primary"
                onClick={() => openDownloadModal(data.id, data.title)}
                title="Выбрать файлы и скачать"
              >
                <ArrowDownToLine className="h-4 w-4" />
                Скачать
              </Button>
              {data.magnet && (
                <Button
                  variant="secondary"
                  loading={adding}
                  onClick={() => add(data.id, { preferMagnet: true })}
                  title="Быстро добавить все файлы через magnet"
                >
                  <Magnet className="h-4 w-4" />
                  Magnet
                </Button>
              )}
              {data.magnet && (
                <Button
                  variant="ghost"
                  onClick={() => {
                    void openUrl(data.magnet!);
                    toast.info("Magnet-ссылка открыта во внешнем приложении");
                  }}
                >
                  Открыть внешне
                </Button>
              )}
              <Button
                variant="ghost"
                onClick={toggleFavorite}
                title={favorite ? "Убрать из избранного" : "В избранное"}
                className="ml-auto"
              >
                <Heart className={clsx("h-4 w-4", favorite && "fill-danger text-danger")} />
                {favorite ? "В избранном" : "В избранное"}
              </Button>
            </div>

            <div className="mt-4 flex flex-wrap gap-4 rounded-lg border border-border bg-surface-2/50 px-4 py-3 text-sm">
              <Stat
                label="Размер"
                value={data.stats.size_bytes ? formatSize(data.stats.size_bytes) : "—"}
              />
              <Stat
                label="Сиды"
                value={
                  <span className="flex items-center gap-1 text-success">
                    <ArrowUp className="h-3.5 w-3.5" />
                    {data.stats.seeders ?? "—"}
                  </span>
                }
              />
              <Stat
                label="Личи"
                value={
                  <span className="flex items-center gap-1 text-muted">
                    <Users className="h-3.5 w-3.5" />
                    {data.stats.leechers ?? "—"}
                  </span>
                }
              />
              <Stat
                label="Скачали"
                value={data.stats.completed != null ? formatNumber(data.stats.completed) : "—"}
              />
              {data.stats.registered && <Stat label="Добавлен" value={data.stats.registered} />}
              {!data.has_torrent_file && !data.magnet && <Badge tone="warn">Файл недоступен</Badge>}
            </div>

            <div className="mt-5">
              <ContentBlocks blocks={data.body} />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[11px] text-faint">{label}</span>
      <span className="font-medium text-text">{value}</span>
    </div>
  );
}
