// Шапка страницы с кнопкой «Назад» (появляется, когда есть куда возвращаться).

import { ArrowLeft } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui";
import { useAppStore, useCanGoBack } from "@/store";

/** Кнопка «Назад»: видна только если в стеке есть предыдущий маршрут. */
export function BackButton({ label = "Назад" }: { label?: string }) {
  const canGoBack = useCanGoBack();
  const back = useAppStore((s) => s.back);
  if (!canGoBack) return null;
  return (
    <Button variant="ghost" onClick={back} className="px-2">
      <ArrowLeft className="h-4 w-4" />
      {label}
    </Button>
  );
}

/** Заголовок раздела с «Назад» и произвольными действиями справа. */
export function PageHeader({ title, actions }: { title: string; actions?: ReactNode }) {
  return (
    <header className="flex items-center gap-3 border-b border-border px-5 py-3">
      <BackButton />
      <h1 className="text-base font-semibold text-text">{title}</h1>
      {actions && <div className="ml-auto flex items-center gap-2">{actions}</div>}
    </header>
  );
}
