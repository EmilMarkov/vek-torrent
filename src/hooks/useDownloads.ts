// Общий стор загрузок: единый слушатель push-событий + явная загрузка снимка.
//
// Слушатель ставится один раз на всё приложение и лишь принимает события (не
// запускает qBittorrent). Явный запуск sidecar происходит только когда
// пользователь открывает вкладку «Загрузки» (см. useDownloadsSnapshot).

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";

import { api } from "@/lib/api";
import type { DownloadItem, DownloadsUpdate, TransferSummary } from "@/lib/types";

interface DownloadsStore {
  items: DownloadItem[];
  transfer: TransferSummary | null;
  loading: boolean;
  error: string | null;
  applyEvent: (update: DownloadsUpdate) => void;
  setLoading: (loading: boolean) => void;
  setSnapshot: (items: DownloadItem[], transfer: TransferSummary) => void;
  setError: (message: string) => void;
}

export const useDownloadsStore = create<DownloadsStore>((set) => ({
  items: [],
  transfer: null,
  loading: false,
  error: null,
  applyEvent: (update) =>
    set({ items: update.items, transfer: update.transfer, loading: false, error: null }),
  setLoading: (loading) => set({ loading }),
  setSnapshot: (items, transfer) => set({ items, transfer, loading: false, error: null }),
  setError: (message) => set({ loading: false, error: message }),
}));

let listenerAttached = false;

/** Ставит единый слушатель событий загрузок (идемпотентно). */
export function useDownloadsListener() {
  useEffect(() => {
    if (listenerAttached) return;
    listenerAttached = true;
    // Слушатель живёт всё время работы приложения — снимать не нужно.
    void listen<DownloadsUpdate>("downloads:update", (event) => {
      useDownloadsStore.getState().applyEvent(event.payload);
    });
  }, []);
}

/** Запускает sidecar и загружает стартовый снимок (для вкладки «Загрузки»). */
export function useDownloadsSnapshot() {
  useEffect(() => {
    let disposed = false;
    const store = useDownloadsStore.getState();
    store.setLoading(true);
    (async () => {
      try {
        const [items, transfer] = await Promise.all([api.downloads(), api.transfer()]);
        if (!disposed) useDownloadsStore.getState().setSnapshot(items, transfer);
      } catch (error) {
        if (!disposed) {
          useDownloadsStore
            .getState()
            .setError(error instanceof Error ? error.message : String(error));
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, []);
}
