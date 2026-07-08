// Тонкая обёртка над Tauri-командами бэкенда.

import { invoke } from "@tauri-apps/api/core";

import type {
  AddOptions,
  AppConfig,
  AppStatus,
  CaptchaAnswer,
  DownloadItem,
  FavoriteItem,
  ForumGroup,
  HistoryItem,
  LoginOutcome,
  SearchPage,
  SearchRequest,
  SessionInfo,
  TopicPage,
  TorrentFilesPreview,
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
  topicFiles: (topicId: number) => call<TorrentFilesPreview>("topic_files", { topicId }),
  addFromTopic: (topicId: number, options: AddOptions) =>
    call<string>("add_from_topic", { topicId, options }),
  addUrl: (url: string, options: AddOptions) => call<string>("add_url", { url, options }),
  pause: (hashes: string[]) => call<void>("pause", { hashes }),
  resume: (hashes: string[]) => call<void>("resume", { hashes }),
  remove: (hashes: string[], deleteFiles: boolean) => call<void>("remove", { hashes, deleteFiles }),

  favorites: () => call<FavoriteItem[]>("favorites"),
  isFavorite: (topicId: number) => call<boolean>("is_favorite", { topicId }),
  addFavorite: (topicId: number) => call<void>("add_favorite", { topicId }),
  removeFavorite: (topicId: number) => call<void>("remove_favorite", { topicId }),
  clearFavoriteUpdate: (topicId: number) => call<void>("clear_favorite_update", { topicId }),
  checkFavorites: () => call<FavoriteItem[]>("check_favorites"),
  history: () => call<HistoryItem[]>("history"),
  removeHistory: (topicId: number) => call<void>("remove_history", { topicId }),
  clearHistory: () => call<void>("clear_history"),

  status: () => call<AppStatus>("status"),
  startEngine: () => call<void>("start_engine"),
  stopEngine: () => call<void>("stop_engine"),
  takePendingDeeplink: () => call<number | null>("take_pending_deeplink"),
  restartApi: () => call<void>("restart_api"),
};
