// Кнопка «В папку»: добавление/удаление раздачи из пользовательских папок.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Folder as FolderIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { clsx } from "clsx";

import { toast } from "@/components/Toaster";
import { Button } from "@/components/ui";
import { api, ApiError } from "@/lib/api";

export function FolderMenuButton({ topicId, title }: { topicId: number; title: string }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();
  const { data: folders } = useQuery({ queryKey: ["folders"], queryFn: api.folders });

  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onEsc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onClick);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("mousedown", onClick);
      document.removeEventListener("keydown", onEsc);
    };
  }, [open]);

  const inFolders = new Set(
    (folders ?? []).filter((f) => f.topics.some((t) => t.topicId === topicId)).map((f) => f.id),
  );

  const toggle = async (folderId: string, folderName: string) => {
    // Быстрые повторные клики принимают решение по устаревшему кешу —
    // блокируем до завершения операции и обновления списка папок.
    if (busy) return;
    setBusy(true);
    try {
      if (inFolders.has(folderId)) {
        await api.removeTopicFromFolder(folderId, topicId);
        toast.info(`Убрано из папки «${folderName}»`);
      } else {
        await api.addTopicToFolder(folderId, topicId, title);
        toast.success(`Добавлено в папку «${folderName}»`);
      }
      await queryClient.invalidateQueries({ queryKey: ["folders"] });
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div ref={ref} className="relative">
      <Button
        variant="ghost"
        onClick={() => setOpen((v) => !v)}
        title="Добавить в папку"
        className={clsx(inFolders.size > 0 && "text-accent")}
      >
        <FolderIcon className={clsx("h-4 w-4", inFolders.size > 0 && "fill-accent/25")} />В папку
      </Button>
      {open && (
        <div className="absolute top-full right-0 z-30 mt-1 w-64 rounded-lg border border-border bg-surface-3 p-1 shadow-lg">
          {(folders ?? []).length === 0 ? (
            <p className="px-3 py-2 text-xs text-faint">
              Папок пока нет — создайте их на странице «Папки».
            </p>
          ) : (
            (folders ?? []).map((folder) => (
              <button
                key={folder.id}
                onClick={() => void toggle(folder.id, folder.name)}
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
              >
                <FolderIcon
                  className="h-4 w-4 shrink-0"
                  style={{ color: folder.category?.color ?? "var(--color-faint)" }}
                />
                <span className="min-w-0 flex-1 truncate">{folder.name}</span>
                {inFolders.has(folder.id) && <Check className="h-4 w-4 shrink-0 text-accent" />}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
