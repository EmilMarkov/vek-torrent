// Страница «Свои торренты»: импорт сторонних .torrent-файлов, их скачивание,
// добавление в папки и удаление.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { File as FileIcon, HardDriveDownload, Plus, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { FolderMenuButton } from "@/components/FolderMenuButton";
import { PageHeader } from "@/components/PageHeader";
import { toast } from "@/components/Toaster";
import { Button, EmptyState, Input, Spinner } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { formatDate, formatSize } from "@/lib/format";
import type { ExternalTorrentItem } from "@/lib/types";

export function ExternalTorrentsPage() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery({
    queryKey: ["external-torrents"],
    queryFn: api.externalTorrents,
  });
  const [importing, setImporting] = useState(false);
  const [query, setQuery] = useState("");

  const refresh = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ["external-torrents"] }),
      queryClient.invalidateQueries({ queryKey: ["folders"] }),
    ]);

  const importFiles = async () => {
    const paths = await open({
      multiple: true,
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });
    if (!paths) return;
    const list = Array.isArray(paths) ? paths : [paths];
    setImporting(true);
    let added = 0;
    try {
      for (const path of list) {
        try {
          await api.addExternalTorrent(path);
          added += 1;
        } catch (error) {
          toast.error(error instanceof ApiError ? error.message : String(error));
        }
      }
      if (added > 0) {
        toast.success(added === 1 ? "Торрент добавлен" : `Добавлено торрентов: ${added}`);
        await refresh();
      }
    } finally {
      setImporting(false);
    }
  };

  const all = useMemo(() => data ?? [], [data]);
  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? all.filter((t) => t.name.toLowerCase().includes(q)) : all;
  }, [all, query]);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Свои торренты"
        actions={
          <Button variant="primary" loading={importing} onClick={() => void importFiles()}>
            <Plus className="h-4 w-4" />
            Добавить .torrent
          </Button>
        }
      />

      {all.length > 0 && (
        <div className="flex items-center gap-3 border-b border-border px-5 py-3">
          <div className="relative max-w-xs flex-1">
            <FileIcon className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-faint" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Поиск по названию…"
              className="pl-9"
            />
          </div>
          <span className="ml-auto text-xs text-faint">
            {visible.length} из {all.length}
          </span>
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex justify-center py-16">
            <Spinner className="h-6 w-6" />
          </div>
        ) : all.length === 0 ? (
          <EmptyState
            icon={<FileIcon className="h-10 w-10" />}
            title="Своих торрентов пока нет"
            hint="Импортируйте .torrent-файлы с диска — их можно скачивать во встроенном движке и раскладывать по папкам."
          />
        ) : visible.length === 0 ? (
          <EmptyState
            icon={<FileIcon className="h-10 w-10" />}
            title="Ничего не найдено"
            hint="Попробуйте изменить запрос."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border/60 px-3">
            {visible.map((torrent) => (
              <ExternalRow key={torrent.id} torrent={torrent} onChanged={refresh} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function ExternalRow({
  torrent,
  onChanged,
}: {
  torrent: ExternalTorrentItem;
  onChanged: () => void;
}) {
  const [downloading, setDownloading] = useState(false);

  const download = async () => {
    // Папка сохранения: выбранная пользователем либо каталог по умолчанию
    // из настроек (если диалог отменён — используем дефолт).
    const dir = await open({ directory: true, title: "Папка для скачивания (необязательно)" });
    const savePath = typeof dir === "string" ? dir : null;
    setDownloading(true);
    try {
      await api.downloadExternalTorrent(torrent.id, { savePath, preferMagnet: false });
      toast.success("Добавлено в загрузки");
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      setDownloading(false);
    }
  };

  const remove = async () => {
    await api.removeExternalTorrent(torrent.id);
    toast.info("Торрент удалён");
    onChanged();
  };

  return (
    <div className="group flex items-center gap-3 py-2.5">
      <FileIcon className="h-5 w-5 shrink-0 text-faint" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium text-text">{torrent.name}</div>
        <div className="mt-0.5 text-xs text-faint">
          {formatSize(torrent.size)} · добавлен {formatDate(torrent.addedAt)}
        </div>
      </div>
      <Button variant="ghost" loading={downloading} onClick={() => void download()} title="Скачать">
        <HardDriveDownload className="h-4 w-4" />
        Скачать
      </Button>
      <FolderMenuButton target={{ kind: "external", externalId: torrent.id }} />
      <Button
        variant="ghost"
        onClick={() => void remove()}
        title="Удалить"
        className="opacity-0 group-hover:opacity-100"
      >
        <Trash2 className="h-4 w-4 text-danger" />
      </Button>
    </div>
  );
}
