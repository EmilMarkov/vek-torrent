// Простые всплывающие уведомления.

import { clsx } from "clsx";
import { CheckCircle2, Info, XCircle } from "lucide-react";
import { useEffect } from "react";
import { create } from "zustand";

type ToastKind = "success" | "error" | "info";

interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

interface ToastStore {
  toasts: Toast[];
  push: (kind: ToastKind, message: string) => void;
  dismiss: (id: number) => void;
}

let nextId = 1;

const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  push: (kind, message) =>
    set((state) => ({ toasts: [...state.toasts, { id: nextId++, kind, message }] })),
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

/** Императивный доступ к уведомлениям из любого места. */
export const toast = {
  success: (m: string) => useToastStore.getState().push("success", m),
  error: (m: string) => useToastStore.getState().push("error", m),
  info: (m: string) => useToastStore.getState().push("info", m),
};

const ICONS = {
  success: <CheckCircle2 className="h-4 w-4 text-success" />,
  error: <XCircle className="h-4 w-4 text-danger" />,
  info: <Info className="h-4 w-4 text-info" />,
};

export function Toaster() {
  const toasts = useToastStore((s) => s.toasts);
  return (
    <div className="pointer-events-none fixed right-4 bottom-10 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  );
}

function ToastItem({ toast: t }: { toast: Toast }) {
  const dismiss = useToastStore((s) => s.dismiss);
  useEffect(() => {
    const timer = setTimeout(() => dismiss(t.id), 4000);
    return () => clearTimeout(timer);
  }, [t.id, dismiss]);

  return (
    <div
      className={clsx(
        "pointer-events-auto flex max-w-sm items-start gap-2 rounded-lg border border-border",
        "bg-surface-2 px-3 py-2 text-sm text-text shadow-lg",
      )}
    >
      {ICONS[t.kind]}
      <span className="flex-1">{t.message}</span>
    </div>
  );
}
