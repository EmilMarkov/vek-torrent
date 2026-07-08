// Добавление раздачи в загрузки с уведомлениями и переходом на вкладку.

import { useState } from "react";

import { toast } from "@/components/Toaster";
import { api, ApiError } from "@/lib/api";
import type { AddOptions } from "@/lib/types";
import { useAppStore } from "@/store";

export function useAddDownload() {
  const [adding, setAdding] = useState(false);
  const setView = useAppStore((s) => s.setView);

  const add = async (topicId: number, options: AddOptions = {}) => {
    setAdding(true);
    try {
      await api.addFromTopic(topicId, options);
      toast.success("Добавлено в загрузки");
      setView("downloads");
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setAdding(false);
    }
  };

  return { add, adding };
}

function describeError(error: unknown): string {
  if (error instanceof ApiError) {
    switch (error.code) {
      case "engine_error":
        return "Торрент-движок недоступен. Попробуйте перезапустить его в настройках.";
      case "not_authenticated":
        return "Требуется вход на rutracker (см. Настройки).";
      default:
        return error.message;
    }
  }
  return String(error);
}
