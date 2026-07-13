// Тонкая обёртка над Tauri-командами бэкенда.

import { invoke } from "@tauri-apps/api/core";

import type {
  AddOptions,
  AppConfig,
  AppStatus,
  CaptchaAnswer,
  CategoryItem,
  ChangeEventItem,
  DownloadItem,
  ExternalTorrentItem,
  FavoriteItem,
  FileVersionInfo,
  FolderItem,
  ForumGroup,
  HistoryItem,
  LoginOutcome,
  MirrorStatus,
  PatchInfo,
  SearchPage,
  SearchRequest,
  SessionInfo,
  TopicPage,
  TorrentFilesPreview,
  TransferSummary,
  VersionMatch,
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
  regenerateApiToken: () => call<string>("regenerate_api_token"),
  checkMirrors: () => call<MirrorStatus[]>("check_mirrors"),

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
  saveTorrent: (topicId: number, path: string) =>
    call<string | null>("save_torrent", { topicId, path }),
  pause: (hashes: string[]) => call<void>("pause", { hashes }),
  resume: (hashes: string[]) => call<void>("resume", { hashes }),
  remove: (hashes: string[], deleteFiles: boolean) => call<void>("remove", { hashes, deleteFiles }),

  userCategories: () => call<CategoryItem[]>("user_categories"),
  addUserCategory: (name: string, color: string, forumIds: number[]) =>
    call<CategoryItem>("add_user_category", { name, color, forumIds }),
  updateUserCategory: (id: string, name: string, color: string, forumIds: number[]) =>
    call<void>("update_user_category", { id, name, color, forumIds }),
  removeUserCategory: (id: string) => call<void>("remove_user_category", { id }),

  folders: () => call<FolderItem[]>("folders"),
  addFolder: (name: string, categoryId: string | null) =>
    call<void>("add_folder", { name, categoryId }),
  updateFolder: (id: string, name: string, categoryId: string | null) =>
    call<void>("update_folder", { id, name, categoryId }),
  removeFolder: (id: string) => call<void>("remove_folder", { id }),
  addTopicToFolder: (folderId: string, topicId: number, title: string) =>
    call<void>("add_topic_to_folder", { folderId, topicId, title }),
  removeTopicFromFolder: (folderId: string, topicId: number) =>
    call<void>("remove_topic_from_folder", { folderId, topicId }),
  addExternalToFolder: (folderId: string, externalId: string) =>
    call<void>("add_external_to_folder", { folderId, externalId }),
  removeExternalFromFolder: (folderId: string, externalId: string) =>
    call<void>("remove_external_from_folder", { folderId, externalId }),

  externalTorrents: () => call<ExternalTorrentItem[]>("external_torrents"),
  addExternalTorrent: (path: string) => call<ExternalTorrentItem>("add_external_torrent", { path }),
  removeExternalTorrent: (id: string) => call<void>("remove_external_torrent", { id }),
  downloadExternalTorrent: (id: string, options: AddOptions) =>
    call<string>("download_external_torrent", { id, options }),

  favorites: () => call<FavoriteItem[]>("favorites"),
  isFavorite: (topicId: number) => call<boolean>("is_favorite", { topicId }),
  addFavorite: (topicId: number) => call<void>("add_favorite", { topicId }),
  removeFavorite: (topicId: number) => call<void>("remove_favorite", { topicId }),
  clearFavoriteUpdate: (topicId: number) => call<void>("clear_favorite_update", { topicId }),
  checkFavorites: () => call<FavoriteItem[]>("check_favorites"),
  favoriteHistory: (topicId: number) => call<ChangeEventItem[]>("favorite_history", { topicId }),
  trackedVersions: (topicId: number) => call<FileVersionInfo[]>("tracked_versions", { topicId }),
  computePatch: (topicId: number, baseAt: number) =>
    call<PatchInfo>("compute_patch", { topicId, baseAt }),
  detectVersion: (topicId: number, dir: string) =>
    call<VersionMatch[]>("detect_version", { topicId, dir }),
  downloadPatch: (topicId: number, baseAt: number, options: AddOptions) =>
    call<string>("download_patch", { topicId, baseAt, options }),
  history: () => call<HistoryItem[]>("history"),
  removeHistory: (topicId: number) => call<void>("remove_history", { topicId }),
  clearHistory: () => call<void>("clear_history"),

  status: () => call<AppStatus>("status"),
  startEngine: () => call<void>("start_engine"),
  stopEngine: () => call<void>("stop_engine"),
  takePendingDeeplink: () => call<number | null>("take_pending_deeplink"),
  restartApi: () => call<void>("restart_api"),
};
