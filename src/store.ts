// Глобальное состояние навигации (Zustand).

import { create } from "zustand";

export type View = "search" | "downloads" | "settings";

interface AppState {
  view: View;
  /** Открытая раздача (перекрывает основной вид), либо null. */
  topicId: number | null;
  setView: (view: View) => void;
  openTopic: (id: number) => void;
  closeTopic: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  view: "search",
  topicId: null,
  setView: (view) => set({ view, topicId: null }),
  openTopic: (topicId) => set({ topicId }),
  closeTopic: () => set({ topicId: null }),
}));
