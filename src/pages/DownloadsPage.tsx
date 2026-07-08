// Страница загрузок: живой список из событий бэкенда с управлением.

import { clsx } from "clsx";
import {
  ArrowDown,
  ArrowUp,
  Ban,
  Download,
  Pause,
  Play,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import { useState } from "react";

import { PageHeader } from "@/components/PageHeader";
import { toast } from "@/components/Toaster";
import { Badge, Button, EmptyState, Spinner } from "@/components/ui";
import { useDownloadsSnapshot, useDownloadsStore } from "@/hooks/useDownloads";
import { api } from "@/lib/api";
import { formatEta, formatSize, formatSpeed } from "@/lib/format";
import type { DownloadItem, TorrentState } from "@/lib/types";

const STATE_LABELS: Record<TorrentState, string> = {
  downloading: "Загрузка",
  uploading: "Раздача",
  queued: "В очереди",
  paused: "Пауза",
  checking: "Проверка",
  metadata: "Метаданные",
  moving: "Перемещение",
  error: "Ошибка",
  unknown: "—",
};

const STATE_TONE: Record<TorrentState, "neutral" | "success" | "warn" | "danger" | "accent"> = {
  downloading: "accent",
  uploading: "success",
  queued: "neutral",
  paused: "neutral",
  checking: "warn",
  metadata: "warn",
  moving: "warn",
  error: "danger",
  unknown: "neutral",
};

export function DownloadsPage() {
  useDownloadsSnapshot();
  const items = useDownloadsStore((s) => s.items);
  const loading = useDownloadsStore((s) => s.loading);
  const error = useDownloadsStore((s) => s.error);

  return (
    <div className="flex h-full flex-col">
      <PageHeader title="Загрузки" />
      <div className="min-h-0 flex-1 overflow-y-auto">
        {loading && items.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <Spinner className="h-6 w-6" />
          </div>
        ) : error ? (
          <EmptyState
            icon={<TriangleAlert className="h-10 w-10 text-warn" />}
            title="Движок загрузок недоступен"
            hint={error}
          />
        ) : items.length === 0 ? (
          <EmptyState
            icon={<Download className="h-10 w-10" />}
            title="Нет активных загрузок"
            hint="Найдите раздачу на вкладке «Поиск» и добавьте её — она появится здесь."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border/60 px-3">
            {items.map((item) => (
              <DownloadRow key={item.hash} item={item} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function DownloadRow({ item }: { item: DownloadItem }) {
  const [busy, setBusy] = useState(false);
  const percent = Math.round(item.progress * 100);
  const paused = item.state === "paused";
  const done = item.progress >= 1;

  const run = async (action: () => Promise<void>, errMsg: string) => {
    setBusy(true);
    try {
      await action();
    } catch {
      toast.error(errMsg);
    } finally {
      setBusy(false);
    }
  };

  const remove = async (deleteFiles: boolean) => {
    const question = deleteFiles
      ? "Удалить раздачу вместе с файлами?"
      : "Убрать раздачу из списка (файлы останутся)?";
    if (!window.confirm(question)) return;
    await run(() => api.remove([item.hash], deleteFiles), "Не удалось удалить");
  };

  return (
    <div className="py-3">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-medium text-text">{item.name}</span>
            <Badge tone={STATE_TONE[item.state]}>{STATE_LABELS[item.state]}</Badge>
            {item.category && <Badge tone="neutral">{item.category}</Badge>}
          </div>

          <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-surface-3">
            <div
              className={clsx(
                "h-full rounded-full transition-all",
                item.state === "error" ? "bg-danger" : done ? "bg-success" : "bg-accent",
              )}
              style={{ width: `${percent}%` }}
            />
          </div>

          <div className="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-faint">
            <span className="text-text/70">
              {percent}% · {formatSize(item.size)}
            </span>
            {!done && item.state === "downloading" && (
              <>
                <span className="flex items-center gap-1 text-success">
                  <ArrowDown className="h-3 w-3" />
                  {formatSpeed(item.dlspeed)}
                </span>
                <span>осталось {formatEta(item.eta)}</span>
              </>
            )}
            {item.upspeed > 0 && (
              <span className="flex items-center gap-1 text-info">
                <ArrowUp className="h-3 w-3" />
                {formatSpeed(item.upspeed)}
              </span>
            )}
            <span title="Сиды / личи">
              {item.numSeeds} сид · {item.numLeechs} лич
            </span>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1">
          {busy ? (
            <Spinner className="mx-2 h-4 w-4" />
          ) : (
            <>
              {paused ? (
                <Button
                  variant="ghost"
                  onClick={() => run(() => api.resume([item.hash]), "Не удалось запустить")}
                  title="Запустить"
                >
                  <Play className="h-4 w-4" />
                </Button>
              ) : (
                <Button
                  variant="ghost"
                  onClick={() => run(() => api.pause([item.hash]), "Не удалось приостановить")}
                  title="Пауза"
                >
                  <Pause className="h-4 w-4" />
                </Button>
              )}
              <Button variant="ghost" onClick={() => remove(false)} title="Убрать из списка">
                <Ban className="h-4 w-4" />
              </Button>
              <Button variant="ghost" onClick={() => remove(true)} title="Удалить с файлами">
                <Trash2 className="h-4 w-4 text-danger" />
              </Button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
