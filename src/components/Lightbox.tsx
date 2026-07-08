// Просмотр изображения на весь экран внутри приложения (без внешних переходов).

import { X } from "lucide-react";
import { create } from "zustand";

interface LightboxStore {
  src: string | null;
  open: (src: string) => void;
  close: () => void;
}

const useLightboxStore = create<LightboxStore>((set) => ({
  src: null,
  open: (src) => set({ src }),
  close: () => set({ src: null }),
}));

/** Открыть изображение на весь экран. */
export function openLightbox(src: string) {
  useLightboxStore.getState().open(src);
}

export function Lightbox() {
  const src = useLightboxStore((s) => s.src);
  const close = useLightboxStore((s) => s.close);

  if (!src) return null;

  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/85 p-6"
      onClick={close}
    >
      <button
        onClick={close}
        title="Закрыть"
        className="absolute top-4 right-4 rounded-lg bg-surface-2/80 p-2 text-text hover:bg-surface-3"
      >
        <X className="h-5 w-5" />
      </button>
      <img
        src={src}
        alt=""
        className="max-h-full max-w-full rounded-lg object-contain"
        onClick={(e) => e.stopPropagation()}
      />
    </div>
  );
}
