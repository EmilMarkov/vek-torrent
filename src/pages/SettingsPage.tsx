// Настройки: аккаунт rutracker (с капчей), движок, загрузки, внешний API.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, FolderOpen, LogIn, LogOut, RefreshCw } from "lucide-react";
import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { CaptchaModal } from "@/components/CaptchaModal";
import { PageHeader } from "@/components/PageHeader";
import { toast } from "@/components/Toaster";
import { Button, Input, Spinner, Toggle } from "@/components/ui";
import { useStatus } from "@/hooks/useStatus";
import { api, ApiError } from "@/lib/api";
import type { AppConfig, CaptchaAnswer, CaptchaChallenge } from "@/lib/types";

const MIRRORS = ["https://rutracker.org", "https://rutracker.net", "https://rutracker.nl"];

export function SettingsPage() {
  const { data: initial, isLoading } = useQuery({ queryKey: ["config"], queryFn: api.getConfig });

  return (
    <div className="flex h-full flex-col">
      <PageHeader title="Настройки" />
      {isLoading || !initial ? (
        <div className="flex flex-1 items-center justify-center">
          <Spinner className="h-6 w-6" />
        </div>
      ) : (
        // Форма инициализирует локальное состояние из props без эффекта.
        <SettingsForm initial={initial} />
      )}
    </div>
  );
}

function SettingsForm({ initial }: { initial: AppConfig }) {
  const queryClient = useQueryClient();
  const { data: status } = useStatus();

  const [config, setConfig] = useState<AppConfig>(initial);
  const [saving, setSaving] = useState(false);
  const [loggingIn, setLoggingIn] = useState(false);
  const [captcha, setCaptcha] = useState<{ challenge: CaptchaChallenge; dataUrl: string } | null>(
    null,
  );

  const patch = (updater: (c: AppConfig) => AppConfig) => setConfig((c) => updater(c));

  const save = async (): Promise<boolean> => {
    setSaving(true);
    try {
      await api.setConfig(config);
      await queryClient.invalidateQueries({ queryKey: ["config"] });
      toast.success("Настройки сохранены");
      return true;
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
      return false;
    } finally {
      setSaving(false);
    }
  };

  const performLogin = async (answer?: CaptchaAnswer) => {
    setLoggingIn(true);
    try {
      const result = await api.login(answer);
      if (result.kind === "ok") {
        setCaptcha(null);
        toast.success("Вход выполнен");
        await queryClient.invalidateQueries({ queryKey: ["status"] });
      } else {
        const image = await api.fetchImage(result.challenge.img_url);
        setCaptcha({ challenge: result.challenge, dataUrl: image.dataUrl });
      }
    } catch (error) {
      toast.error(error instanceof ApiError ? error.message : String(error));
    } finally {
      setLoggingIn(false);
    }
  };

  const saveAndLogin = async () => {
    if (await save()) await performLogin();
  };

  const logout = async () => {
    await api.logout();
    await queryClient.invalidateQueries({ queryKey: ["status"] });
    toast.info("Выход выполнен");
  };

  const pickSaveDir = async () => {
    const path = await open({ directory: true });
    if (typeof path === "string")
      patch((c) => ({ ...c, downloads: { ...c.downloads, default_save_path: path } }));
  };

  const setCategoryPath = (key: keyof AppConfig["downloads"]["category_paths"], value: string) =>
    patch((c) => ({
      ...c,
      downloads: {
        ...c.downloads,
        category_paths: { ...c.downloads.category_paths, [key]: value },
      },
    }));

  const pickCategoryDir = async (key: keyof AppConfig["downloads"]["category_paths"]) => {
    const path = await open({ directory: true });
    if (typeof path === "string") setCategoryPath(key, path);
  };

  const restartApi = async () => {
    if (await save()) {
      try {
        await api.restartApi();
        await queryClient.invalidateQueries({ queryKey: ["status"] });
        toast.success(config.api.enabled ? "API перезапущен" : "API остановлен");
      } catch (error) {
        toast.error(error instanceof ApiError ? error.message : String(error));
      }
    }
  };

  return (
    <div className="min-h-0 flex-1 overflow-y-auto">
      <div className="mx-auto flex max-w-2xl flex-col gap-6 px-6 py-6">
        <Section
          title="Аккаунт rutracker"
          subtitle={
            status?.loggedIn
              ? `В сети как ${status.username ?? "пользователь"}`
              : "Не выполнен вход"
          }
        >
          <Row label="Логин">
            <Input
              value={config.rutracker.username}
              onChange={(e) =>
                patch((c) => ({ ...c, rutracker: { ...c.rutracker, username: e.target.value } }))
              }
            />
          </Row>
          <Row label="Пароль">
            <Input
              type="password"
              value={config.rutracker.password}
              placeholder="оставьте пустым, чтобы не менять"
              onChange={(e) =>
                patch((c) => ({ ...c, rutracker: { ...c.rutracker, password: e.target.value } }))
              }
            />
          </Row>
          <Row label="Зеркало">
            <Input
              list="mirrors"
              value={config.rutracker.mirror}
              onChange={(e) =>
                patch((c) => ({ ...c, rutracker: { ...c.rutracker, mirror: e.target.value } }))
              }
            />
            <datalist id="mirrors">
              {MIRRORS.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </Row>
          <Row label="Прокси" hint="socks5:// или http://, необязательно">
            <Input
              value={config.rutracker.proxy}
              placeholder="напр. socks5://127.0.0.1:9050"
              onChange={(e) =>
                patch((c) => ({ ...c, rutracker: { ...c.rutracker, proxy: e.target.value } }))
              }
            />
          </Row>
          <div className="flex gap-2">
            <Button variant="primary" loading={loggingIn} onClick={saveAndLogin}>
              <LogIn className="h-4 w-4" />
              Сохранить и войти
            </Button>
            {status?.loggedIn && (
              <Button variant="ghost" onClick={logout}>
                <LogOut className="h-4 w-4" />
                Выйти
              </Button>
            )}
          </div>
        </Section>

        <Section
          title="Торрент-движок"
          subtitle={
            status?.engineRunning ? "Встроенный движок запущен" : "Встроенный движок остановлен"
          }
        >
          <Row label="Порт для входящих соединений" hint="0 — выбрать автоматически">
            <Input
              type="number"
              value={config.engine.listen_port}
              onChange={(e) =>
                patch((c) => ({
                  ...c,
                  engine: { ...c.engine, listen_port: Number(e.target.value) || 0 },
                }))
              }
            />
          </Row>
          <Toggle
            checked={config.engine.autostart}
            onChange={(v) => patch((c) => ({ ...c, engine: { ...c.engine, autostart: v } }))}
            label="Запускать движок вместе с приложением"
          />
          <div className="flex gap-2">
            <Button
              variant="secondary"
              onClick={() =>
                api
                  .startEngine()
                  .then(() => toast.success("Движок запускается…"))
                  .catch((e) => toast.error(e instanceof ApiError ? e.message : String(e)))
              }
            >
              Запустить
            </Button>
            <Button
              variant="ghost"
              onClick={() => api.stopEngine().then(() => toast.info("Движок остановлен"))}
            >
              Остановить
            </Button>
          </div>
        </Section>

        <Section title="Загрузки">
          <Row label="Каталог по умолчанию" hint="Куда сохранять новые загрузки">
            <div className="flex gap-2">
              <Input
                value={config.downloads.default_save_path}
                placeholder="по умолчанию — папка приложения"
                onChange={(e) =>
                  patch((c) => ({
                    ...c,
                    downloads: { ...c.downloads, default_save_path: e.target.value },
                  }))
                }
              />
              <Button variant="secondary" onClick={pickSaveDir}>
                <FolderOpen className="h-4 w-4" />
              </Button>
            </div>
          </Row>
          <Toggle
            checked={config.downloads.add_stopped}
            onChange={(v) =>
              patch((c) => ({ ...c, downloads: { ...c.downloads, add_stopped: v } }))
            }
            label="Добавлять новые загрузки на паузе"
          />

          <div className="mt-1 flex flex-col gap-2 border-t border-border pt-3">
            <span className="text-xs font-medium text-muted">Каталоги по категориям</span>
            <span className="text-[11px] text-faint">
              Раздачи определённой категории сохраняются в свой каталог (если задан).
            </span>
            {(
              [
                ["films", "Фильмы"],
                ["games", "Игры"],
                ["music", "Музыка"],
                ["books", "Книги"],
              ] as const
            ).map(([key, label]) => (
              <div key={key} className="flex items-center gap-2">
                <span className="w-16 shrink-0 text-xs text-muted">{label}</span>
                <Input
                  value={config.downloads.category_paths[key]}
                  placeholder="как каталог по умолчанию"
                  onChange={(e) => setCategoryPath(key, e.target.value)}
                />
                <Button variant="secondary" onClick={() => pickCategoryDir(key)}>
                  <FolderOpen className="h-4 w-4" />
                </Button>
              </div>
            ))}
          </div>
        </Section>

        <Section
          title="Внешний API"
          subtitle="REST API для интеграций с другими программами (Swagger на /docs)"
        >
          <Toggle
            checked={config.api.enabled}
            onChange={(v) => patch((c) => ({ ...c, api: { ...c.api, enabled: v } }))}
            label="Включить внешний API"
          />
          <Row label="Адрес и порт">
            <div className="flex gap-2">
              <Input
                value={config.api.host}
                onChange={(e) => patch((c) => ({ ...c, api: { ...c.api, host: e.target.value } }))}
              />
              <Input
                type="number"
                className="w-28"
                value={config.api.port}
                onChange={(e) =>
                  patch((c) => ({ ...c, api: { ...c.api, port: Number(e.target.value) || 0 } }))
                }
              />
            </div>
          </Row>
          <Row label="Токен доступа" hint="Bearer-токен для заголовка Authorization">
            <div className="flex gap-2">
              <Input value={config.api.token} readOnly className="font-mono text-xs" />
              <Button
                variant="secondary"
                onClick={() => {
                  void navigator.clipboard.writeText(config.api.token);
                  toast.success("Токен скопирован");
                }}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
          </Row>
          <Button variant="secondary" onClick={restartApi}>
            <RefreshCw className="h-4 w-4" />
            Применить настройки API
          </Button>
        </Section>

        <div className="sticky bottom-0 -mx-6 border-t border-border bg-bg/80 px-6 py-3 backdrop-blur">
          <Button variant="primary" loading={saving} onClick={save}>
            Сохранить настройки
          </Button>
        </div>
      </div>

      {captcha && (
        <CaptchaModal
          dataUrl={captcha.dataUrl}
          loading={loggingIn}
          onSubmit={(value) =>
            performLogin({
              sid: captcha.challenge.sid,
              code_field: captcha.challenge.code_field,
              value,
            })
          }
          onCancel={() => setCaptcha(null)}
        />
      )}
    </div>
  );
}

function Section({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-border bg-surface p-5">
      <div className="mb-4">
        <h2 className="text-base font-semibold text-text">{title}</h2>
        {subtitle && <p className="mt-0.5 text-xs text-faint">{subtitle}</p>}
      </div>
      <div className="flex flex-col gap-3">{children}</div>
    </section>
  );
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-muted">{label}</span>
      {children}
      {hint && <span className="text-[11px] text-faint">{hint}</span>}
    </label>
  );
}
