// Небольшие переиспользуемые UI-примитивы в тёмной теме.

import { clsx } from "clsx";
import { Check, ChevronDown, Loader2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  loading?: boolean;
}

const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary: "bg-accent text-white hover:bg-accent-hover disabled:bg-accent/50",
  secondary: "bg-surface-3 text-text hover:bg-border-strong disabled:opacity-50",
  ghost: "bg-transparent text-muted hover:bg-surface-2 hover:text-text disabled:opacity-50",
  danger: "bg-danger/15 text-danger hover:bg-danger/25 disabled:opacity-50",
};

export function Button({
  variant = "secondary",
  loading = false,
  className,
  children,
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      className={clsx(
        "inline-flex items-center justify-center gap-2 rounded-lg px-3.5 py-2 text-sm font-medium",
        "transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/60",
        "disabled:cursor-not-allowed",
        BUTTON_VARIANTS[variant],
        className,
      )}
      disabled={disabled || loading}
      {...props}
    >
      {loading && <Loader2 className="h-4 w-4 animate-spin" />}
      {children}
    </button>
  );
}

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={clsx(
        "w-full rounded-lg border border-border bg-surface-2 px-3 py-2 text-sm text-text",
        "placeholder:text-faint focus:border-accent/70 focus:outline-none focus:ring-2 focus:ring-accent/25",
        className,
      )}
      {...props}
    />
  );
}

export function Spinner({ className }: { className?: string }) {
  return <Loader2 className={clsx("animate-spin text-muted", className)} />;
}

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}

/** Кастомный выпадающий список — единый вид на всех ОС (не нативный select). */
export function Select<T extends string>({
  value,
  onChange,
  options,
  className,
}: {
  value: T;
  onChange: (value: T) => void;
  options: SelectOption<T>[];
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onEsc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onEsc);
    };
  }, [open]);

  const current = options.find((o) => o.value === value);

  return (
    <div ref={ref} className={clsx("relative", className)}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between gap-2 rounded-lg border border-border bg-surface-2 px-3 py-2 text-sm text-text hover:border-border-strong focus:border-accent/70 focus:outline-none"
      >
        <span className="truncate">{current?.label ?? "—"}</span>
        <ChevronDown
          className={clsx("h-4 w-4 shrink-0 text-faint transition-transform", open && "rotate-180")}
        />
      </button>
      {open && (
        <div className="absolute top-full right-0 left-0 z-20 mt-1 max-h-64 overflow-y-auto rounded-lg border border-border bg-surface-3 py-1 shadow-lg">
          {options.map((o) => (
            <button
              key={o.value}
              type="button"
              onClick={() => {
                onChange(o.value);
                setOpen(false);
              }}
              className={clsx(
                "flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-surface-2",
                o.value === value ? "text-accent" : "text-text",
              )}
            >
              <Check
                className={clsx("h-3.5 w-3.5", o.value === value ? "opacity-100" : "opacity-0")}
              />
              <span className="truncate">{o.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: ReactNode;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-2.5 text-sm text-text select-none">
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={clsx(
          "inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors",
          checked ? "bg-accent" : "bg-surface-3",
        )}
      >
        <span
          className={clsx(
            "inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform",
            checked ? "translate-x-4" : "translate-x-1",
          )}
        />
      </button>
      {label}
    </label>
  );
}

export function EmptyState({
  icon,
  title,
  hint,
}: {
  icon?: ReactNode;
  title: string;
  hint?: ReactNode;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
      {icon && <div className="text-faint">{icon}</div>}
      <p className="text-sm font-medium text-muted">{title}</p>
      {hint && <p className="max-w-sm text-xs text-faint">{hint}</p>}
    </div>
  );
}

export function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "success" | "warn" | "danger" | "accent";
}) {
  const tones = {
    neutral: "bg-surface-3 text-muted",
    success: "bg-success/15 text-success",
    warn: "bg-warn/15 text-warn",
    danger: "bg-danger/15 text-danger",
    accent: "bg-accent-soft text-accent",
  };
  return (
    <span
      className={clsx(
        "inline-flex items-center rounded-md px-1.5 py-0.5 text-[11px] font-medium",
        tones[tone],
      )}
    >
      {children}
    </span>
  );
}
