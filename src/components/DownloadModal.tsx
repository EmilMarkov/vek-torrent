// Модалка выбора файлов перед скачиванием (как в qBittorrent).
//
// Глобальный стор: любой компонент вызывает openDownloadModal(topicId, title),
// а сама модалка монтируется один раз в App.

import { useQuery } from "@tanstack/react-query";
import { CheckSquare, Square, TriangleAlert } from "lucide-react";
import { useMemo, useState } from "react";
import { create } from "zustand";

import { toast } from "@/components/Toaster";
import { Button, Spinner, Toggle } from "@/components/ui";
import { api, ApiError } from "@/lib/api";
import { formatSize } from "@/lib/format";
import { useAppStore } from "@/store";

interface ModalStore {
  topicId: number | null;
  title: string;
  open: (topicId: number, title: string) => void;
  close: () => void;
}

const useModalStore = create<ModalStore>((set) => ({
  topicId: null,
  title: "",
  open: (topicId, title) => set({ topicId, title }),
  close: () => set({ topicId: null }),
}));

/** Открыть модалку выбора файлов для раздачи. */
export function openDownloadModal(topicId: number, title: string) {
  useModalStore.getState().open(topicId, title);
}

export function DownloadModal() {
  const topicId = useModalStore((s) => s.topicId);
  const title = useModalStore((s) => s.title);
  const close = useModalStore((s) => s.close);

  if (topicId === null) return null;
  return <ModalBody topicId={topicId} title={title} onClose={close} />;
}

function ModalBody({
  topicId,
  title,
  onClose,
}: {
  topicId: number;
  title: string;
  onClose: () => void;
}) {
  const setView = useAppStore((s) => s.setView);
  // null — «пользователь ничего не менял» → выбраны все файлы по умолчанию.
  const [custom, setCustom] = useState<Set<number> | null>(null);
  const [stopped, setStopped] = useState(false);
  const [adding, setAdding] = useState(false);

  const { data, isLoading, error } = useQuery({
    queryKey: ["topic-files", topicId],
    queryFn: () => api.topicFiles(topicId),
    retry: false,
  });

  const allIndices = useMemo(() => new Set(data?.files.map((f) => f.index) ?? []), [data]);
  // Эффективный выбор: пользовательский либо «все» (без эффектов синхронизации).
  const selected = custom ?? allIndices;

  const selectedSize = useMemo(
    () =>
      data ? data.files.filter((f) => selected.has(f.index)).reduce((s, f) => s + f.size, 0) : 0,
    [data, selected],
  );

  const toggle = (index: number) =>
    setCustom((prev) => {
      const next = new Set(prev ?? allIndices);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });

  const allSelected = data ? selected.size === data.files.length : false;
  const toggleAll = () => setCustom(allSelected ? new Set() : new Set(allIndices));

  const confirm = async () => {
    if (!data || selected.size === 0) return;
    setAdding(true);
    try {
      const onlyFiles = allSelected ? null : [...selected];
      await api.addFromTopic(topicId, { onlyFiles, stopped });
      toast.success("Добавлено в загрузки");
      onClose();
      setView("downloads");
    } catch (e) {
      toast.error(e instanceof ApiError ? describe(e) : String(e));
    } finally {
      setAdding(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onClose}
    >
      <div
        className="flex max-h-[80vh] w-full max-w-2xl flex-col rounded-xl border border-border bg-surface"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="border-b border-border px-5 py-3">
          <h3 className="truncate text-base font-semibold text-text">Выбор файлов</h3>
          <p className="mt-0.5 truncate text-xs text-faint">{data?.name || title}</p>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto">
          {isLoading ? (
            <div className="flex h-40 items-center justify-center">
              <Spinner className="h-6 w-6" />
            </div>
          ) : error || !data ? (
            <div className="flex h-40 flex-col items-center justify-center gap-2 px-6 text-center">
              <TriangleAlert className="h-8 w-8 text-warn" />
              <p className="text-sm text-muted">Не удалось получить список файлов</p>
              <p className="text-xs text-faint">
                {error instanceof ApiError ? describe(error) : "Проверьте вход на rutracker."}
              </p>
            </div>
          ) : (
            <div className="p-2">
              <button
                onClick={toggleAll}
                className="mb-1 flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs font-medium text-muted hover:bg-surface-2"
              >
                {allSelected ? (
                  <CheckSquare className="h-4 w-4 text-accent" />
                ) : (
                  <Square className="h-4 w-4" />
                )}
                {allSelected ? "Снять все" : "Выбрать все"} ({data.files.length})
              </button>
              {data.files.map((file) => {
                const checked = selected.has(file.index);
                return (
                  <label
                    key={file.index}
                    className="flex cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm hover:bg-surface-2"
                  >
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={() => toggle(file.index)}
                      className="accent-accent"
                    />
                    <span className="min-w-0 flex-1 truncate text-text/90" title={file.path}>
                      {file.path}
                    </span>
                    <span className="shrink-0 text-xs text-faint">{formatSize(file.size)}</span>
                  </label>
                );
              })}
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 border-t border-border px-5 py-3">
          <Toggle checked={stopped} onChange={setStopped} label="На паузе" />
          <div className="ml-auto flex items-center gap-3">
            {data && (
              <span className="text-xs text-faint">
                {selected.size} из {data.files.length} · {formatSize(selectedSize)}
              </span>
            )}
            <Button variant="ghost" onClick={onClose}>
              Отмена
            </Button>
            <Button
              variant="primary"
              loading={adding}
              disabled={!data || selected.size === 0}
              onClick={confirm}
            >
              Скачать
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function describe(error: ApiError): string {
  if (error.code === "not_authenticated") return "Требуется вход на rutracker (см. Настройки).";
  if (error.code === "engine_error") return "Торрент-движок недоступен.";
  return error.message;
}
