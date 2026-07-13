// Навигация приложения на основе стека истории.
//
// Любой переход (клик по разделу в sidebar, открытие раздачи) кладёт маршрут в
// стек; «Назад» снимает верхний маршрут и возвращает на предыдущий с его
// сохранённым состоянием. Так «Назад» работает единообразно по всей программе.

import { create } from "zustand";

export type MainView =
  "search" | "downloads" | "favorites" | "folders" | "categories" | "history" | "settings";

export type Route =
  | { kind: MainView }
  | { kind: "topic"; topicId: number }
  | { kind: "tracked-history"; topicId: number; title: string };

const MAX_STACK = 50;

function sameRoute(a: Route, b: Route): boolean {
  if (a.kind !== b.kind) return false;
  if ("topicId" in a && "topicId" in b) return a.topicId === b.topicId;
  return true;
}

interface AppState {
  stack: Route[];
  /** Перейти на маршрут (добавить в стек). Дубликаты подряд игнорируются. */
  navigate: (route: Route) => void;
  /** Перейти на основной раздел. */
  setView: (view: MainView) => void;
  /** Открыть страницу раздачи. */
  openTopic: (topicId: number) => void;
  /** Вернуться на предыдущий маршрут. */
  back: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  stack: [{ kind: "search" }],
  navigate: (route) =>
    set((state) => {
      const top = state.stack[state.stack.length - 1];
      if (sameRoute(top, route)) return state;
      const stack = [...state.stack, route].slice(-MAX_STACK);
      return { stack };
    }),
  setView: (view) =>
    set((state) => {
      const top = state.stack[state.stack.length - 1];
      if (top.kind === view) return state;
      const route: Route = { kind: view };
      return { stack: [...state.stack, route].slice(-MAX_STACK) };
    }),
  openTopic: (topicId) =>
    set((state) => {
      const top = state.stack[state.stack.length - 1];
      if (top.kind === "topic" && top.topicId === topicId) return state;
      const route: Route = { kind: "topic", topicId };
      return { stack: [...state.stack, route].slice(-MAX_STACK) };
    }),
  back: () =>
    set((state) => {
      if (state.stack.length <= 1) return state;
      return { stack: state.stack.slice(0, -1) };
    }),
}));

/** Текущий (верхний) маршрут. */
export function useCurrentRoute(): Route {
  return useAppStore((s) => s.stack[s.stack.length - 1]);
}

/** Доступен ли переход «Назад». */
export function useCanGoBack(): boolean {
  return useAppStore((s) => s.stack.length > 1);
}
