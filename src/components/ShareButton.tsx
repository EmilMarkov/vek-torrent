// Кнопка «Поделиться»: ссылки rutracker/приложения, magnet-ссылка.

import { Copy, Link2, Magnet, Share2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { toast } from "@/components/Toaster";
import { Button } from "@/components/ui";
import { rutrackerTopicUrl } from "@/lib/rutracker";

interface Props {
  topicId: number;
  /** Magnet-ссылка раздачи (пункт копирования показывается при наличии). */
  magnet?: string | null;
}

export function ShareButton({ topicId, magnet }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

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

  const rutrackerUrl = rutrackerTopicUrl(topicId);
  const appUrl = `vektorrent://topic/${topicId}`;

  const copy = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`${label} скопирована`);
    } catch {
      toast.error("Не удалось скопировать");
    }
    setOpen(false);
  };

  return (
    <div ref={ref} className="relative">
      <Button variant="ghost" onClick={() => setOpen((v) => !v)} title="Поделиться">
        <Share2 className="h-4 w-4" />
        Поделиться
      </Button>
      {open && (
        <div className="absolute top-full right-0 z-30 mt-1 w-64 rounded-lg border border-border bg-surface-3 p-1 shadow-lg">
          <button
            onClick={() => copy(rutrackerUrl, "Ссылка rutracker")}
            className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
          >
            <Copy className="h-4 w-4 text-faint" />
            Копировать ссылку rutracker
          </button>
          <button
            onClick={() => copy(appUrl, "Ссылка на приложение")}
            className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
          >
            <Link2 className="h-4 w-4 text-faint" />
            Копировать ссылку приложения
          </button>
          {magnet && (
            <button
              onClick={() => copy(magnet, "Magnet-ссылка")}
              className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
            >
              <Magnet className="h-4 w-4 text-faint" />
              Копировать magnet-ссылку
            </button>
          )}
        </div>
      )}
    </div>
  );
}
