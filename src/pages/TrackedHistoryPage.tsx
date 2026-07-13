// История изменений отслеживаемой раздачи + скачивание патча.
//
// Патч — это выбор в актуальной раздаче только тех файлов, которые
// добавились или изменились относительно версии, скачанной пользователем.

import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  FileClock,
  FolderOpen,
  FolderSearch,
  HardDriveDownload,
  PackageOpen,
  X,
} from "lucide-react";
import { useRef, useState } from "react";
import { clsx } from "clsx";
import { open } from "@tauri-apps/plugin-dialog";

import { toast } from "@/components/Toaster";
import { Badge, Button, EmptyState, Select, Spinner } from "@/components/ui";
import { useFavorites } from "@/hooks/useLibrary";
import { api, ApiError } from "@/lib/api";
import { formatSize } from "@/lib/format";
import type { PatchInfo, VersionMatch } from "@/lib/types";
import { useAppStore } from "@/store";

/** Unix-время → локальные дата и время (события истории). */
function formatDateTime(unix: number): string {
  if (!unix || unix <= 0) return "—";
  return new Date(unix * 1000).toLocaleString("ru-RU", {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function TrackedHistoryPage({ topicId, title }: { topicId: number; title: string }) {
  const back = useAppStore((s) => s.back);
  const openTopic = useAppStore((s) => s.openTopic);
  const [patchOpen, setPatchOpen] = useState(false);

  const { data: history, isLoading } = useQuery({
    queryKey: ["favorite-history", topicId],
    queryFn: () => api.favoriteHistory(topicId),
  });
  const { data: versions } = useQuery({
    queryKey: ["tracked-versions", topicId],
    queryFn: () => api.trackedVersions(topicId),
  });
  // Название в маршруте — снимок на момент перехода; раздача могла
  // переименоваться, поэтому показываем актуальное из отслеживаемого.
  const { data: favorites } = useFavorites();
  const currentTitle = favorites?.find((f) => f.topicId === topicId)?.title ?? title;

  const canPatch = (versions ?? []).length >= 2;

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-3 border-b border-border px-5 py-3">
        <Button variant="ghost" onClick={back} className="px-2">
          <ArrowLeft className="h-4 w-4" />
          Назад
        </Button>
        <FileClock className="h-5 w-5 shrink-0 text-accent" />
        <button
          onClick={() => openTopic(topicId)}
          className="min-w-0 truncate text-left text-base font-semibold text-text hover:text-accent"
          title="Открыть раздачу"
        >
          {currentTitle}
        </button>
        <div className="ml-auto">
          <Button
            variant="primary"
            disabled={!canPatch}
            onClick={() => setPatchOpen(true)}
            title={
              canPatch
                ? "Скачать только изменённые файлы"
                : "Нужно минимум две сохранённые версии файлов"
            }
          >
            <PackageOpen className="h-4 w-4" />
            Скачать патч…
          </Button>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto flex max-w-3xl flex-col gap-5 px-6 py-5">
          {/* Версии файлов. */}
          {(versions ?? []).length > 0 && (
            <section className="rounded-xl border border-border bg-surface p-4">
              <h2 className="mb-1 text-sm font-semibold text-text">Версии файлов</h2>
              <p className="mb-3 text-[11px] text-faint">
                Снимки списка файлов раздачи. Первая (самая ранняя из сохранённых) — базовая точка
                отсчёта для патчей; она сохраняется при добавлении в отслеживаемое и не означает
                изменения. Патч можно посчитать от любой из версий.
              </p>
              <div className="flex flex-col divide-y divide-border/60">
                {(versions ?? [])
                  .slice()
                  .reverse()
                  .map((v) => (
                    <div key={v.index} className="flex items-center gap-3 py-2 text-sm">
                      <Badge tone={v.index === (versions ?? []).length - 1 ? "accent" : "neutral"}>
                        v{v.index + 1}
                      </Badge>
                      <span className="text-text">{formatDateTime(v.at)}</span>
                      {v.index === 0 && <span className="text-xs text-faint">самая ранняя</span>}
                      <span className="ml-auto text-xs text-faint">
                        файлов: {v.fileCount} · {formatSize(v.totalSize)}
                      </span>
                    </div>
                  ))}
              </div>
            </section>
          )}

          {/* История изменений. */}
          {isLoading ? (
            <div className="flex justify-center py-16">
              <Spinner className="h-6 w-6" />
            </div>
          ) : (history ?? []).length === 0 ? (
            <EmptyState
              icon={<FileClock className="h-10 w-10" />}
              title="Изменений пока не зафиксировано"
              hint="История наполняется при проверках обновлений (вручную или автоматически каждые 3 часа)."
            />
          ) : (
            <section className="flex flex-col gap-3">
              {(history ?? []).map((event, i) => (
                <div key={i} className="rounded-xl border border-border bg-surface p-4">
                  <div className="mb-2 text-xs font-medium text-muted">
                    {formatDateTime(event.at)}
                  </div>
                  {event.changes.length === 0 ? (
                    <p className="text-sm text-faint">Обновление без деталей</p>
                  ) : (
                    <ul className="flex flex-col gap-1 text-sm text-text">
                      {event.changes.map((change, j) => (
                        <li key={j}>• {change}</li>
                      ))}
                    </ul>
                  )}
                </div>
              ))}
            </section>
          )}
        </div>
      </div>

      {patchOpen && <PatchModal topicId={topicId} onClose={() => setPatchOpen(false)} />}
    </div>
  );
}

// ── Модалка патча ────────────────────────────────────────────────────────────

function PatchModal({ topicId, onClose }: { topicId: number; onClose: () => void }) {
  const { data: versions } = useQuery({
    queryKey: ["tracked-versions", topicId],
    queryFn: () => api.trackedVersions(topicId),
  });

  // Версия адресуется временем фиксации (стабильно к вытеснению старых).
  const [baseAt, setBaseAt] = useState<string>("");
  // Отбрасываем устаревшие ответы computePatch при быстрой смене версии.
  const requestSeq = useRef(0);
  const [detecting, setDetecting] = useState(false);
  const [matches, setMatches] = useState<VersionMatch[] | null>(null);
  const [patch, setPatch] = useState<PatchInfo | null>(null);
  const [computing, setComputing] = useState(false);
  const [savePath, setSavePath] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);

  const versionOptions = [
    { value: "", label: "Выберите версию…" },
    ...(versions ?? []).map((v) => ({
      value: String(v.at),
      label: `v${v.index + 1} · ${formatDateTime(v.at)} · файлов: ${v.fileCount}`,
    })),
  ];

  const selectBase = async (value: string) => {
    const seq = ++requestSeq.current;
    setBaseAt(value);
    setPatch(null);
    if (value === "") return;
    setComputing(true);
    try {
      const result = await api.computePatch(topicId, Number(value));
      if (seq !== requestSeq.current) return; // выбор уже сменился
      setPatch(result);
    } catch (error) {
      if (seq !== requestSeq.current) return;
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      if (seq === requestSeq.current) setComputing(false);
    }
  };

  // Определение версии по локальной папке: сравнение путей и размеров файлов.
  const detectFromFolder = async () => {
    const dir = await open({ directory: true, title: "Папка со скачанной раздачей" });
    if (typeof dir !== "string") return;
    setDetecting(true);
    setMatches(null);
    try {
      const result = await api.detectVersion(topicId, dir);
      setMatches(result);
      const best = result[0];
      if (best && best.matched > 0) {
        toast.success(
          `Похоже на v${best.version + 1}: совпало ${best.matched} из ${best.total} файлов`,
        );
        await selectBase(String(best.at));
      } else {
        toast.info("Совпадений с сохранёнными версиями не найдено");
      }
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      setDetecting(false);
    }
  };

  const pickSaveDir = async () => {
    const dir = await open({ directory: true, title: "Куда сохранить патч" });
    if (typeof dir === "string") setSavePath(dir);
  };

  const download = async () => {
    if (baseAt === "" || downloading) return;
    setDownloading(true);
    try {
      await api.downloadPatch(topicId, Number(baseAt), {
        savePath,
        preferMagnet: false,
      });
      toast.success("Патч добавлен в загрузки");
      onClose();
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
      setDownloading(false);
    }
  };

  const downloadable = (patch?.files ?? []).filter((f) => f.kind !== "removed");
  const removed = (patch?.files ?? []).filter((f) => f.kind === "removed");

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-6">
      <div className="flex max-h-[85vh] w-full max-w-xl flex-col rounded-xl border border-border bg-surface shadow-xl">
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 className="text-base font-semibold text-text">Скачать патч</h2>
          {/* Во время добавления патча в движок закрытие не отменяет операцию —
              блокируем, чтобы не создавать ложное ощущение отмены. */}
          <Button variant="ghost" onClick={onClose} disabled={downloading} className="px-2">
            <X className="h-4 w-4" />
          </Button>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-xs font-medium text-muted">
              Какую версию вы скачивали раньше?
            </span>
            <div className="flex gap-2">
              <Select
                className="flex-1"
                value={baseAt}
                onChange={(v) => void selectBase(v)}
                options={versionOptions}
              />
              <Button
                variant="secondary"
                loading={detecting}
                onClick={() => void detectFromFolder()}
              >
                <FolderSearch className="h-4 w-4" />
                Определить по папке
              </Button>
            </div>
            <span className="text-[11px] text-faint">
              «Определить по папке» сравнит файлы выбранной папки (имена и размеры) с сохранёнными
              версиями и подберёт наиболее похожую.
            </span>
          </div>

          {matches && matches.length > 0 && (
            <div className="flex flex-col gap-1 rounded-lg border border-border bg-surface-2/50 px-3 py-2">
              {matches.slice(0, 4).map((m) => (
                <button
                  key={m.version}
                  onClick={() => void selectBase(String(m.at))}
                  className={clsx(
                    "flex items-center justify-between text-xs hover:text-accent",
                    String(m.at) === baseAt ? "text-accent" : "text-muted",
                  )}
                >
                  <span>
                    v{m.version + 1} · {formatDateTime(m.at)}
                  </span>
                  <span>
                    совпало {m.matched}/{m.total}
                  </span>
                </button>
              ))}
            </div>
          )}

          {computing && (
            <div className="flex justify-center py-6">
              <Spinner className="h-5 w-5" />
            </div>
          )}

          {patch && !computing && (
            <div className="flex min-h-0 flex-col gap-2">
              <div className="text-sm text-text">
                {patch.files.length === 0 ? (
                  "Изменённых файлов нет — вы на актуальной версии."
                ) : downloadable.length === 0 ? (
                  `Скачивать нечего: изменения — только удаление файлов (${removed.length}) из раздачи.`
                ) : (
                  <>
                    Будет скачано <b>{downloadable.length}</b> файлов (
                    {formatSize(patch.downloadSize)})
                    {removed.length > 0 && (
                      <span className="text-faint"> · удалено из раздачи: {removed.length}</span>
                    )}
                  </>
                )}
              </div>
              {patch.files.length > 0 && (
                <div className="max-h-48 overflow-y-auto rounded-lg border border-border bg-surface-2/50">
                  {patch.files.map((f) => (
                    <div
                      key={`${f.kind}:${f.path}`}
                      className="flex items-center gap-2 px-3 py-1 text-xs"
                    >
                      <Badge
                        tone={
                          f.kind === "added" ? "success" : f.kind === "changed" ? "warn" : "danger"
                        }
                      >
                        {f.kind === "added" ? "новый" : f.kind === "changed" ? "изменён" : "удалён"}
                      </Badge>
                      <span
                        className={clsx(
                          "min-w-0 flex-1 truncate",
                          f.kind === "removed" ? "text-faint line-through" : "text-text",
                        )}
                      >
                        {f.path}
                      </span>
                      <span className="shrink-0 text-faint">{formatSize(f.size)}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center gap-2 border-t border-border px-5 py-3">
          <Button variant="secondary" onClick={() => void pickSaveDir()}>
            <FolderOpen className="h-4 w-4" />
            {savePath ? "Папка выбрана" : "Папка сохранения…"}
          </Button>
          {savePath && (
            <span className="min-w-0 flex-1 truncate text-xs text-faint" title={savePath}>
              {savePath}
            </span>
          )}
          <div className="ml-auto flex gap-2">
            <Button variant="ghost" onClick={onClose} disabled={downloading}>
              Отмена
            </Button>
            <Button
              variant="primary"
              loading={downloading}
              disabled={baseAt === "" || !patch || downloadable.length === 0}
              onClick={() => void download()}
            >
              <HardDriveDownload className="h-4 w-4" />
              Скачать патч
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
