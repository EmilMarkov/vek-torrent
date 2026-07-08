// Модальное окно ввода капчи при входе на rutracker.

import { useState } from "react";

import { Button, Input } from "@/components/ui";

interface Props {
  dataUrl: string;
  loading: boolean;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}

export function CaptchaModal({ dataUrl, loading, onSubmit, onCancel }: Props) {
  const [value, setValue] = useState("");

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onCancel}
    >
      <div
        className="w-full max-w-sm rounded-xl border border-border bg-surface p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-base font-semibold text-text">Подтвердите вход</h3>
        <p className="mt-1 text-xs text-faint">
          rutracker запросил капчу. Введите символы с картинки.
        </p>

        <div className="mt-4 flex justify-center rounded-lg border border-border bg-white p-2">
          <img src={dataUrl} alt="Капча" className="max-h-24" />
        </div>

        <form
          className="mt-4 flex flex-col gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (value.trim()) onSubmit(value.trim());
          }}
        >
          <Input
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="Код с картинки"
            autoFocus
          />
          <div className="flex gap-2">
            <Button type="submit" variant="primary" loading={loading} disabled={!value.trim()}>
              Войти
            </Button>
            <Button type="button" variant="ghost" onClick={onCancel}>
              Отмена
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
