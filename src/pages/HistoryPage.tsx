// Страница истории скачиваний.

import { useQueryClient } from "@tanstack/react-query";
import { ArrowDownToLine, History, Trash2, X } from "lucide-react";

import { openDownloadModal } from "@/components/DownloadModal";
import { PageHeader } from "@/components/PageHeader";
import { Button, EmptyState, Spinner } from "@/components/ui";
import { useHistory } from "@/hooks/useLibrary";
import { api } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { useAppStore } from "@/store";

export function HistoryPage() {
  const queryClient = useQueryClient();
  const openTopic = useAppStore((s) => s.openTopic);
  const { data, isLoading } = useHistory();

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["history"] });

  const items = data ?? [];

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="История скачиваний"
        actions={
          items.length > 0 && (
            <Button
              variant="ghost"
              onClick={async () => {
                await api.clearHistory();
                refresh();
              }}
            >
              <X className="h-4 w-4" />
              Очистить
            </Button>
          )
        }
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex h-full items-center justify-center">
            <Spinner className="h-6 w-6" />
          </div>
        ) : items.length === 0 ? (
          <EmptyState
            icon={<History className="h-10 w-10" />}
            title="История пуста"
            hint="Здесь будут раздачи, которые вы добавляли в загрузки."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border/60 px-3">
            {items.map((item) => (
              <div
                key={`${item.topicId}-${item.addedAt}`}
                className="group flex items-center gap-3 py-2.5"
              >
                <button
                  onClick={() => openTopic(item.topicId)}
                  className="min-w-0 flex-1 text-left"
                >
                  <span className="block truncate text-sm font-medium text-text group-hover:text-accent">
                    {item.title}
                  </span>
                  <span className="text-xs text-faint">Добавлено {formatDate(item.addedAt)}</span>
                </button>
                <Button
                  variant="ghost"
                  onClick={() => openDownloadModal(item.topicId, item.title)}
                  title="Скачать снова"
                  className="shrink-0 opacity-0 group-hover:opacity-100"
                >
                  <ArrowDownToLine className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  onClick={async () => {
                    await api.removeHistory(item.topicId);
                    refresh();
                  }}
                  title="Удалить из истории"
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
