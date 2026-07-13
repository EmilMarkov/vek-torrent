// Строка таблицы результатов поиска.
//
// Ширины фиксированных колонок разделены с шапкой таблицы (RESULT_COLS в
// SearchPage) — при изменении здесь синхронизировать с заголовком.

import { clsx } from "clsx";
import { ArrowUp, CheckCircle2, Users } from "lucide-react";

import { DownloadMenu } from "@/components/DownloadMenu";
import type { ClientFilters } from "@/lib/filters";
import { formatDate, formatNumber, formatSize } from "@/lib/format";
import type { SearchResult } from "@/lib/types";
import { useAppStore } from "@/store";

/**
 * Ширины фиксированных колонок таблицы результатов (шапка + строки).
 * Дата скрывается на узких окнах, чтобы названию оставалось место.
 */
export const RESULT_COLS = {
  size: "w-16",
  seeders: "w-12",
  leechers: "w-12",
  downloads: "w-16",
  date: "w-24 max-lg:hidden",
  actions: "w-10",
} as const;

interface Props {
  result: SearchResult;
  /** Частичное обновление клиентских фильтров (клик по автору/разделу). */
  onPatchFilters: (patch: Partial<ClientFilters>) => void;
}

export function ResultRow({ result, onPatchFilters }: Props) {
  const openTopic = useAppStore((s) => s.openTopic);

  return (
    <div className="group flex items-center gap-3 border-b border-border/60 py-2.5">
      <div className="min-w-0 flex-1">
        <button
          onClick={() => openTopic(result.topic_id)}
          className="flex w-full items-center gap-2 text-left"
        >
          {result.approval === "approved" && (
            <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-success" />
          )}
          <span className="truncate text-sm font-medium text-text group-hover:text-accent">
            {result.title}
          </span>
        </button>
        <div className="mt-0.5 flex items-center gap-1.5 text-xs text-faint">
          {result.forum && (
            <button
              onClick={() => onPatchFilters({ forumIds: [result.forum!.id] })}
              className="max-w-[50%] truncate hover:text-accent hover:underline"
              title={`Показать только раздел «${result.forum.name}»`}
            >
              {result.forum.name}
            </button>
          )}
          {result.author && (
            <>
              {result.forum && <span>·</span>}
              <button
                onClick={() => onPatchFilters({ author: result.author! })}
                className="truncate hover:text-accent hover:underline"
                title={`Показать только раздачи автора ${result.author}`}
              >
                {result.author}
              </button>
            </>
          )}
        </div>
      </div>

      <span
        className={clsx(RESULT_COLS.size, "shrink-0 text-right text-xs font-medium text-text/80")}
      >
        {formatSize(result.size_bytes)}
      </span>
      <span
        className={clsx(
          RESULT_COLS.seeders,
          "flex shrink-0 items-center justify-end gap-1 text-xs",
          result.seeders > 0 ? "text-success" : "text-faint",
        )}
        title="Сиды"
      >
        <ArrowUp className="h-3 w-3" />
        {formatNumber(result.seeders)}
      </span>
      <span
        className={clsx(
          RESULT_COLS.leechers,
          "flex shrink-0 items-center justify-end gap-1 text-xs text-muted",
        )}
        title="Личи"
      >
        <Users className="h-3 w-3" />
        {formatNumber(result.leechers)}
      </span>
      <span
        className={clsx(RESULT_COLS.downloads, "shrink-0 text-right text-xs text-muted")}
        title="Скачиваний"
      >
        {formatNumber(result.downloads)}
      </span>
      <span className={clsx(RESULT_COLS.date, "shrink-0 text-right text-xs text-faint")}>
        {formatDate(result.added_unix)}
      </span>

      <div className={clsx(RESULT_COLS.actions, "flex shrink-0 justify-end")}>
        <DownloadMenu
          topicId={result.topic_id}
          title={result.title}
          compact
          className="opacity-0 group-hover:opacity-100"
        />
      </div>
    </div>
  );
}
