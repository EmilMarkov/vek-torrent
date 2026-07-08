// Кнопка «Поделиться»: ссылка rutracker и внутренняя ссылка на приложение.

import { Copy, ExternalLink, Link2, Share2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { toast } from "@/components/Toaster";
import { Button } from "@/components/ui";

const RUTRACKER_BASE = "https://rutracker.org/forum/viewtopic.php?t=";

export function ShareButton({ topicId }: { topicId: number }) {
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

  const rutrackerUrl = `${RUTRACKER_BASE}${topicId}`;
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
          <button
            onClick={() => {
              void openUrl(rutrackerUrl);
              setOpen(false);
            }}
            className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm text-text hover:bg-surface-2"
          >
            <ExternalLink className="h-4 w-4 text-faint" />
            Открыть на rutracker
          </button>
        </div>
      )}
    </div>
  );
}
