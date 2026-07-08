// Тонкая обёртка над Tauri-командами бэкенда.

import { invoke } from "@tauri-apps/api/core";

import type {
  AddOptions,
  AppConfig,
  AppStatus,
  Category,
  CaptchaAnswer,
  DownloadItem,
  ForumGroup,
  LoginOutcome,
  SearchPage,
  SearchRequest,
  SessionInfo,
  TopicPage,
  TransferSummary,
} from "./types";

/** Ошибка команды с машиночитаемым кодом. */
export class ApiError extends Error {
  code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

function isCommandError(value: unknown): value is { code: string; message: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as { message: unknown }).message === "string"
  );
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isCommandError(error)) {
      throw new ApiError(error.code, error.message);
    }
    throw new ApiError("error", typeof error === "string" ? error : String(error));
  }
}

export const api = {
  getConfig: () => call<AppConfig>("get_config"),
  setConfig: (config: AppConfig) => call<void>("set_config", { config }),

  sessionStatus: () => call<SessionInfo>("session_status"),
  login: (captcha?: CaptchaAnswer) => call<LoginOutcome>("login", { captcha: captcha ?? null }),
  logout: () => call<void>("logout"),
  fetchImage: (url: string) => call<{ dataUrl: string }>("fetch_image", { url }),

  search: (request: SearchRequest) => call<SearchPage>("search", { request }),
  topic: (id: number) => call<TopicPage>("topic", { id }),
  categories: () => call<ForumGroup[]>("categories"),

  downloads: () => call<DownloadItem[]>("downloads"),
  transfer: () => call<TransferSummary>("transfer"),
  qbitCategories: () => call<Category[]>("qbit_categories"),
  addFromTopic: (topicId: number, options: AddOptions) =>
    call<void>("add_from_topic", { topicId, options }),
  addUrl: (url: string, options: AddOptions) => call<void>("add_url", { url, options }),
  pause: (hashes: string[]) => call<void>("pause", { hashes }),
  resume: (hashes: string[]) => call<void>("resume", { hashes }),
  remove: (hashes: string[], deleteFiles: boolean) => call<void>("remove", { hashes, deleteFiles }),

  status: () => call<AppStatus>("status"),
  startQbit: () => call<void>("start_qbit"),
  stopQbit: () => call<void>("stop_qbit"),
  restartApi: () => call<void>("restart_api"),
};
