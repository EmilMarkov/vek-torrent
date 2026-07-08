// Обработка внутренних ссылок vektorrent://topic/<id>: переход на раздачу.

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

import { api } from "@/lib/api";
import { useAppStore } from "@/store";

let listenerAttached = false;

export function useDeepLink() {
  const openTopic = useAppStore((s) => s.openTopic);

  useEffect(() => {
    // Холодный старт: приложение открыли ссылкой.
    api
      .takePendingDeeplink()
      .then((id) => {
        if (id != null) openTopic(id);
      })
      .catch(() => {});

    if (listenerAttached) return;
    listenerAttached = true;
    // Приложение уже запущено: ссылка приходит событием.
    void listen<number>("open-topic", (event) => {
      useAppStore.getState().openTopic(event.payload);
    });
  }, [openTopic]);
}
