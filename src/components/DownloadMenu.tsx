// Кнопка «Скачать» с контекстным меню: во встроенный движок или .torrent-файлом.
//
// Меню рендерится порталом в body с fixed-позицией: внутри виртуализированного
// списка результатов absolute-меню перекрывалось бы соседними строками и
// обрезалось скролл-контейнером.

import { ArrowDownToLine, FileDown, HardDriveDownload } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { clsx } from "clsx";
import { save } from "@tauri-apps/plugin-dialog";

import { openDownloadModal } from "@/components/DownloadModal";
import { toast } from "@/components/Toaster";
import { Button } from "@/components/ui";
import { api, ApiError } from "@/lib/api";

interface Props {
  topicId: number;
  title: string;
  /** Компактный вид: только иконка (для строк таблицы результатов). */
  compact?: boolean;
  /** Дополнительные классы кнопки (например, показ по hover в строке). */
  className?: string;
}

export function DownloadMenu({ topicId, title, compact = false, className }: Props) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<{ top: number; right: number } | null>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const toggleOpen = () => {
    if (open) {
      setOpen(false);
      return;
    }
    const rect = wrapRef.current?.getBoundingClientRect();
    if (!rect) return;
    setPosition({ top: rect.bottom + 4, right: window.innerWidth - rect.right });
    setOpen(true);
  };

  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (wrapRef.current?.contains(target) || menuRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onEsc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    // Fixed-меню не следует за прокруткой — закрываем (capture ловит
    // скролл вложенных контейнеров).
    const onScroll = () => setOpen(false);
    document.addEventListener("mousedown", onClick);
    document.addEventListener("keydown", onEsc);
    document.addEventListener("scroll", onScroll, true);
    return () => {
      document.removeEventListener("mousedown", onClick);
      document.removeEventListener("keydown", onEsc);
      document.removeEventListener("scroll", onScroll, true);
    };
  }, [open]);

  const downloadInApp = () => {
    setOpen(false);
    openDownloadModal(topicId, title);
  };

  const saveTorrentFile = async () => {
    setOpen(false);
    // Формат имени как у файлов rutracker; пользователь может поменять в диалоге.
    const path = await save({
      defaultPath: `[rutracker.org].t${topicId}.torrent`,
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });
    if (typeof path !== "string") return;
    // На Linux диалог может вернуть имя без расширения.
    const finalPath = path.endsWith(".torrent") ? path : `${path}.torrent`;
    try {
      await api.saveTorrent(topicId, finalPath);
      toast.success(".torrent-файл сохранён");
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    }
  };

  return (
    <div ref={wrapRef} className="relative">
      <Button
        variant={compact ? "ghost" : "primary"}
        onClick={toggleOpen}
        title="Скачать"
        // hover-классы (opacity-0 и т.п.) не применяем при открытом меню,
        // иначе кнопка-якорь исчезает, когда курсор уходит на меню.
        className={clsx(!open && className)}
      >
        <ArrowDownToLine className="h-4 w-4" />
        {!compact && "Скачать"}
      </Button>
      {open &&
        position &&
        createPortal(
          <div
            ref={menuRef}
            style={{ top: position.top, right: position.right }}
            className="fixed z-50 w-64 rounded-lg border border-border bg-surface-3 p-1 shadow-lg"
          >
            <button
              onClick={downloadInApp}
              className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
            >
              <HardDriveDownload className="h-4 w-4 text-faint" />
              Скачать в приложении…
            </button>
            <button
              onClick={() => void saveTorrentFile()}
              className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
            >
              <FileDown className="h-4 w-4 text-faint" />
              Сохранить .torrent-файл…
            </button>
          </div>,
          document.body,
        )}
    </div>
  );
}
