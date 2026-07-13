// Системные настройки: автозапуск при входе в систему и обновления приложения.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { DownloadCloud, RefreshCw } from "lucide-react";
import { useState } from "react";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";

import { Section } from "@/pages/SettingsPage";
import { toast } from "@/components/Toaster";
import { Button, Spinner, Toggle } from "@/components/ui";

export function SystemSection() {
  return (
    <>
      <AutostartSection />
      <UpdatesSection />
    </>
  );
}

function AutostartSection() {
  const queryClient = useQueryClient();
  const { data: enabled, isLoading } = useQuery({
    queryKey: ["autostart"],
    queryFn: isEnabled,
  });

  const toggle = async (value: boolean) => {
    try {
      if (value) await enable();
      else await disable();
      await queryClient.invalidateQueries({ queryKey: ["autostart"] });
      toast.success(value ? "Автозапуск включён" : "Автозапуск выключен");
    } catch (error) {
      toast.error(`Не удалось изменить автозапуск: ${String(error)}`);
    }
  };

  return (
    <Section title="Система">
      {isLoading ? (
        <Spinner className="h-4 w-4" />
      ) : (
        <Toggle
          checked={enabled ?? false}
          onChange={(v) => void toggle(v)}
          label="Запускать при входе в систему"
        />
      )}
      <span className="text-[11px] text-faint">Применяется сразу, без сохранения настроек.</span>
    </Section>
  );
}

function UpdatesSection() {
  const { data: version } = useQuery({ queryKey: ["app-version"], queryFn: getVersion });
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [available, setAvailable] = useState<string | null>(null);

  const checkForUpdates = async () => {
    setChecking(true);
    setAvailable(null);
    try {
      const update = await check();
      if (update) {
        setAvailable(update.version);
        toast.success(`Доступно обновление ${update.version}`);
      } else {
        toast.info("У вас последняя версия");
      }
    } catch (error) {
      toast.error(`Не удалось проверить обновления: ${String(error)}`);
    } finally {
      setChecking(false);
    }
  };

  const installUpdate = async () => {
    setInstalling(true);
    try {
      const update = await check();
      if (!update) {
        setAvailable(null);
        toast.info("Обновление уже установлено");
        return;
      }
      toast.info("Скачивание обновления…");
      await update.downloadAndInstall();
      // Приложение перезапустится с новой версией.
      await relaunch();
    } catch (error) {
      toast.error(`Не удалось установить обновление: ${String(error)}`);
      setInstalling(false);
    }
  };

  return (
    <Section title="Обновления" subtitle={version ? `Текущая версия: ${version}` : undefined}>
      <div className="flex items-center gap-2">
        <Button variant="secondary" loading={checking} onClick={() => void checkForUpdates()}>
          <RefreshCw className="h-4 w-4" />
          Проверить обновления
        </Button>
        {available && (
          <Button variant="primary" loading={installing} onClick={() => void installUpdate()}>
            <DownloadCloud className="h-4 w-4" />
            Установить {available}
          </Button>
        )}
      </div>
      <span className="text-[11px] text-faint">
        Обновления скачиваются с GitHub Releases и устанавливаются с перезапуском приложения.
      </span>
    </Section>
  );
}
