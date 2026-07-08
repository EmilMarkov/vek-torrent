// Нижняя строка статуса: движок, сессия, скорости, внешний API.

import { ArrowDown, ArrowUp, CircleDot } from "lucide-react";

import { useDownloadsStore } from "@/hooks/useDownloads";
import { useStatus } from "@/hooks/useStatus";
import { formatSpeed } from "@/lib/format";

export function StatusBar() {
  const { data: status } = useStatus();
  const transfer = useDownloadsStore((s) => s.transfer);

  const dot = (ok: boolean) => (ok ? "text-success" : "text-faint");

  return (
    <footer className="flex items-center gap-4 border-t border-border bg-surface px-4 py-1.5 text-xs text-muted">
      <span className="flex items-center gap-1.5" title="Торрент-движок">
        <CircleDot className={`h-3 w-3 ${dot(status?.engineRunning ?? false)}`} />
        Движок{status?.engineRunning ? "" : " остановлен"}
      </span>

      <span className="flex items-center gap-1.5" title="Сессия rutracker">
        <CircleDot className={`h-3 w-3 ${dot(status?.loggedIn ?? false)}`} />
        {status?.loggedIn ? (status.username ?? "в сети") : "не в сети"}
      </span>

      {status?.apiRunning && (
        <span className="flex items-center gap-1.5" title="Внешний API">
          <CircleDot className="h-3 w-3 text-info" />
          API
        </span>
      )}

      <div className="ml-auto flex items-center gap-4">
        <span className="flex items-center gap-1 text-success">
          <ArrowDown className="h-3.5 w-3.5" />
          {formatSpeed(transfer?.dlSpeed ?? 0)}
        </span>
        <span className="flex items-center gap-1 text-info">
          <ArrowUp className="h-3.5 w-3.5" />
          {formatSpeed(transfer?.upSpeed ?? 0)}
        </span>
      </div>
    </footer>
  );
}
