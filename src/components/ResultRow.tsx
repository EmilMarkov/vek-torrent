// Строка результата поиска.

import { clsx } from "clsx";
import { ArrowDownToLine, ArrowUp, CheckCircle2, Users } from "lucide-react";

import { Button } from "@/components/ui";
import { useAddDownload } from "@/hooks/useAddDownload";
import { formatDate, formatNumber, formatSize } from "@/lib/format";
import type { SearchResult } from "@/lib/types";
import { useAppStore } from "@/store";

export function ResultRow({ result }: { result: SearchResult }) {
  const openTopic = useAppStore((s) => s.openTopic);
  const { add, adding } = useAddDownload();

  return (
    <div className="group flex items-center gap-3 border-b border-border/60 py-2.5">
      <button onClick={() => openTopic(result.topic_id)} className="min-w-0 flex-1 text-left">
        <div className="flex items-center gap-2">
          {result.approval === "approved" && (
            <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-success" />
          )}
          <span className="truncate text-sm font-medium text-text group-hover:text-accent">
            {result.title}
          </span>
        </div>
        <div className="mt-0.5 flex items-center gap-3 text-xs text-faint">
          {result.forum && <span className="truncate">{result.forum.name}</span>}
          {result.author && <span className="truncate">· {result.author}</span>}
          <span>· {formatDate(result.added_unix)}</span>
        </div>
      </button>

      <div className="flex shrink-0 items-center gap-4 text-xs">
        <span className="w-16 text-right font-medium text-text/80">
          {formatSize(result.size_bytes)}
        </span>
        <span
          className={clsx(
            "flex w-12 items-center justify-end gap-1",
            result.seeders > 0 ? "text-success" : "text-faint",
          )}
          title="Сиды"
        >
          <ArrowUp className="h-3 w-3" />
          {formatNumber(result.seeders)}
        </span>
        <span className="flex w-12 items-center justify-end gap-1 text-muted" title="Личи">
          <Users className="h-3 w-3" />
          {formatNumber(result.leechers)}
        </span>
      </div>

      <Button
        variant="ghost"
        loading={adding}
        onClick={() => add(result.topic_id)}
        className="shrink-0 opacity-0 group-hover:opacity-100"
        title="Скачать"
      >
        <ArrowDownToLine className="h-4 w-4" />
      </Button>
    </div>
  );
}
