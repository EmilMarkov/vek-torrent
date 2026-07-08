// Данные библиотеки (избранное/история) + слушатель фоновых обновлений.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

import { api } from "@/lib/api";
import type { FavoriteItem } from "@/lib/types";

export function useFavorites() {
  return useQuery({ queryKey: ["favorites"], queryFn: api.favorites });
}

export function useHistory() {
  return useQuery({ queryKey: ["history"], queryFn: api.history });
}

/** Число избранных раздач с обнаруженным обновлением (для индикатора). */
export function useFavoriteUpdatesCount(): number {
  const { data } = useFavorites();
  return (data ?? []).filter((f) => f.hasUpdate).length;
}

let listenerAttached = false;

/** Слушает фоновые проверки обновлений и обновляет кэш избранного (один раз). */
export function useFavoritesListener() {
  const queryClient = useQueryClient();
  useEffect(() => {
    if (listenerAttached) return;
    listenerAttached = true;
    void listen<FavoriteItem[]>("favorites:updated", (event) => {
      queryClient.setQueryData(["favorites"], event.payload);
    });
  }, [queryClient]);
}
