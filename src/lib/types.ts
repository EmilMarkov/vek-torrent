// Типы данных, зеркалящие DTO бэкенда (vek-core / rutracker / engine).

export type ApprovalStatus =
  | "approved"
  | "not_approved"
  | "need_edit"
  | "duplicate"
  | "consumed"
  | "closed"
  | "doubtful"
  | "temporary"
  | "unknown";

export interface ForumRef {
  id: number;
  name: string;
}

export interface SearchResult {
  topic_id: number;
  title: string;
  forum: ForumRef | null;
  author: string | null;
  size_bytes: number;
  seeders: number;
  leechers: number;
  downloads: number;
  added_unix: number;
  approval: ApprovalStatus;
}

export interface SearchPage {
  items: SearchResult[];
  total_found: number;
  offset: number;
  search_id: string | null;
}

export type SortField = "registered" | "title" | "downloads" | "size" | "seeders" | "leechers";
export type SortOrder = "asc" | "desc";

export interface SearchRequest {
  query: string;
  forums: number[];
  author: string | null;
  sort: SortField;
  order: SortOrder;
  offset: number;
  search_id: string | null;
}

export interface ForumEntry {
  id: number;
  name: string;
  depth: number;
}

export interface ForumGroup {
  title: string;
  forums: ForumEntry[];
}

export interface TorrentStats {
  size_bytes: number | null;
  seeders: number | null;
  leechers: number | null;
  completed: number | null;
  registered: string | null;
}

export interface TopicPage {
  id: number;
  title: string;
  forum_path: ForumRef[];
  /** Автор раздачи (ник автора первого поста). */
  author: string | null;
  magnet: string | null;
  has_torrent_file: boolean;
  stats: TorrentStats;
  /** Санированный HTML первого поста в родной разметке rutracker. */
  body_html: string;
}

export type DownloadState = "downloading" | "seeding" | "paused" | "checking" | "error" | "unknown";

export interface DownloadItem {
  hash: string;
  name: string;
  size: number;
  progress: number;
  dlspeed: number;
  upspeed: number;
  eta: number | null;
  state: DownloadState;
  savePath: string;
  numPeers: number;
  downloaded: number;
  uploaded: number;
  finished: boolean;
  error: string | null;
}

export interface TransferSummary {
  dlSpeed: number;
  upSpeed: number;
  active: number;
  total: number;
}

export interface TorrentFile {
  index: number;
  path: string;
  size: number;
}

export interface TorrentFilesPreview {
  hash: string;
  name: string;
  totalSize: number;
  files: TorrentFile[];
}

export interface AddOptions {
  savePath?: string | null;
  stopped?: boolean | null;
  preferMagnet?: boolean;
  onlyFiles?: number[] | null;
}

export interface AppStatus {
  engineRunning: boolean;
  apiRunning: boolean;
  loggedIn: boolean;
  username: string | null;
  activeDownloads: number;
}

export interface FavoriteItem {
  topicId: number;
  title: string;
  addedAt: number;
  lastChecked: number;
  hasUpdate: boolean;
  /** Что именно изменилось (пусто, если детали неизвестны). */
  changes: string[];
  /** Сколько событий в истории изменений. */
  historyCount: number;
}

/** Событие истории изменений отслеживаемой раздачи. */
export interface ChangeEventItem {
  at: number;
  changes: string[];
}

/** Версия списка файлов раздачи (сводка). */
export interface FileVersionInfo {
  index: number;
  at: number;
  fileCount: number;
  totalSize: number;
}

/** Изменение файла в патче. */
export interface FileChangeItem {
  path: string;
  size: number;
  kind: "added" | "changed" | "removed";
}

/** Патч между версией пользователя и актуальной раздачей. */
export interface PatchInfo {
  files: FileChangeItem[];
  downloadSize: number;
  baseAt: number;
}

/** Совпадение локальной папки с версией раздачи. */
export interface VersionMatch {
  version: number;
  at: number;
  matched: number;
  total: number;
}

/** Пользовательская категория: метка для папок + набор разделов rutracker. */
export interface CategoryItem {
  id: string;
  name: string;
  color: string;
  /** Разделы rutracker, объединяемые категорией (для фильтров поиска). */
  forumIds: number[];
}

export interface FolderTopicItem {
  topicId: number;
  title: string;
  addedAt: number;
}

/** Сторонний .torrent, импортированный пользователем. */
export interface ExternalTorrentItem {
  id: string;
  name: string;
  infoHash: string;
  size: number;
  addedAt: number;
}

/** Пользовательская папка с раздачами. */
export interface FolderItem {
  id: string;
  name: string;
  category: CategoryItem | null;
  topics: FolderTopicItem[];
  externals: ExternalTorrentItem[];
  createdAt: number;
}

export interface HistoryItem {
  topicId: number;
  title: string;
  hash: string;
  addedAt: number;
}

export interface SessionInfo {
  logged_in: boolean;
  username: string | null;
}

export interface CaptchaChallenge {
  sid: string;
  code_field: string;
  img_url: string;
}

export interface CaptchaAnswer {
  sid: string;
  code_field: string;
  value: string;
}

export type LoginOutcome = { kind: "ok" } | { kind: "captcha"; challenge: CaptchaChallenge };

// Конфигурация приложения.
export interface RutrackerConfig {
  username: string;
  password: string;
  mirror: string;
  proxy: string;
  auto_mirror: boolean;
}

/** Результат проверки доступности зеркала rutracker. */
export interface MirrorStatus {
  url: string;
  ok: boolean;
  latencyMs: number | null;
  error: string | null;
  current: boolean;
}

export interface EngineConfig {
  listen_port: number;
  autostart: boolean;
}

export interface ApiConfig {
  enabled: boolean;
  host: string;
  port: number;
  token: string;
}

export interface CategoryPaths {
  films: string;
  games: string;
  music: string;
  books: string;
}

export interface DownloadsConfig {
  default_save_path: string;
  add_stopped: boolean;
  category_paths: CategoryPaths;
}

export interface FavoritesConfig {
  /** Сбрасывать метку обновления при открытии раздачи из отслеживаемого. */
  auto_clear_update: boolean;
  /** Отслеживать изменения текста описания. */
  track_description: boolean;
  /** Отслеживать изменения файлов (версии для патчей). */
  track_files: boolean;
}

export interface AppConfig {
  rutracker: RutrackerConfig;
  engine: EngineConfig;
  api: ApiConfig;
  downloads: DownloadsConfig;
  favorites: FavoritesConfig;
}

export interface CommandError {
  code: string;
  message: string;
}

export interface DownloadsUpdate {
  items: DownloadItem[];
  transfer: TransferSummary;
}
