// Страница раздачи: заголовок, статистика, действия и рендер содержимого.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, ArrowUp, ExternalLink, Eye, TriangleAlert, Users } from "lucide-react";
import { clsx } from "clsx";
import { ask } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";

import { DownloadMenu } from "@/components/DownloadMenu";
import { FolderMenuButton } from "@/components/FolderMenuButton";
import { PostBody } from "@/components/PostBody";
import { ShareButton } from "@/components/ShareButton";
import { toast } from "@/components/Toaster";
import { Badge, Button, EmptyState, Spinner } from "@/components/ui";
import { api } from "@/lib/api";
import { formatNumber, formatSize } from "@/lib/format";
import { rutrackerTopicUrl } from "@/lib/rutracker";
import { useAppStore } from "@/store";

export function TopicView({ topicId }: { topicId: number }) {
  const back = useAppStore((s) => s.back);
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

  // Зеркало из настроек: «Открыть на rutracker» должна работать и когда
  // канонический rutracker.org заблокирован.
  const { data: config } = useQuery({ queryKey: ["config"], queryFn: api.getConfig });
  const mirror = config?.rutracker.mirror;

  const toggleFavorite = async () => {
    try {
      if (favorite) {
        // Снятие с отслеживания стирает историю изменений и версии файлов.
        const history = await api.favoriteHistory(topicId);
        if (history.length > 0) {
          const confirmed = await ask(
            `Вместе с раздачей будет безвозвратно удалена история изменений (событий: ${history.length}) и сохранённые версии файлов.`,
            { title: "Перестать отслеживать?", kind: "warning" },
          );
          if (!confirmed) return;
        }
        await api.removeFavorite(topicId);
      } else {
        await api.addFavorite(topicId);
      }
      await queryClient.invalidateQueries({ queryKey: ["is-favorite", topicId] });
      await queryClient.invalidateQueries({ queryKey: ["favorites"] });
      toast.success(favorite ? "Раздача больше не отслеживается" : "Раздача отслеживается");
    } catch {
      toast.error("Не удалось изменить отслеживание");
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
          <div className="mx-auto max-w-6xl px-6 py-5">
            <h1 className="selectable text-xl leading-snug font-semibold text-text">
              {data.title}
            </h1>

            <div className="mt-3 flex flex-wrap items-center gap-2">
              <DownloadMenu topicId={data.id} title={data.title} />
              <Button
                variant="ghost"
                onClick={() => void openUrl(rutrackerTopicUrl(data.id, mirror))}
                title="Открыть страницу раздачи в браузере"
              >
                <ExternalLink className="h-4 w-4" />
                Открыть на rutracker
              </Button>
              <div className="ml-auto flex items-center gap-2">
                <ShareButton topicId={data.id} magnet={data.magnet} />
                <FolderMenuButton target={{ kind: "topic", topicId: data.id, title: data.title }} />
                <Button
                  variant="ghost"
                  onClick={toggleFavorite}
                  title={
                    favorite
                      ? "Перестать отслеживать обновления раздачи"
                      : "Следить за обновлениями раздачи"
                  }
                >
                  <Eye className={clsx("h-4 w-4", favorite && "text-accent")} />
                  {favorite ? "Отслеживается" : "Отслеживать"}
                </Button>
              </div>
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
              {data.author && <Stat label="Автор" value={data.author} />}
              {!data.has_torrent_file && !data.magnet && <Badge tone="warn">Файл недоступен</Badge>}
            </div>

            <div className="mt-5">
              <PostBody html={data.body_html} />
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
