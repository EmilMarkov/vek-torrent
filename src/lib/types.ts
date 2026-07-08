// Типы данных, зеркалящие DTO бэкенда (vek-core / rutracker / qbit).

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

// Блоки содержимого раздачи.
export type Inline =
  | {
      type: "text";
      text: string;
      bold?: boolean;
      italic?: boolean;
      underline?: boolean;
      strike?: boolean;
      color?: string | null;
    }
  | { type: "link"; href: string; text: string; topic_id?: number | null }
  | { type: "break" };

export type ContentBlock =
  | { type: "paragraph"; inlines: Inline[] }
  | { type: "image"; src: string }
  | { type: "spoiler"; title: string; blocks: ContentBlock[] }
  | { type: "quote"; author: string | null; blocks: ContentBlock[] }
  | { type: "code"; text: string }
  | { type: "list"; ordered: boolean; items: ContentBlock[][] }
  | { type: "hr" };

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
  magnet: string | null;
  has_torrent_file: boolean;
  stats: TorrentStats;
  body: ContentBlock[];
}

export type TorrentState =
  | "downloading"
  | "uploading"
  | "queued"
  | "paused"
  | "checking"
  | "metadata"
  | "moving"
  | "error"
  | "unknown";

export interface DownloadItem {
  hash: string;
  name: string;
  size: number;
  progress: number;
  dlspeed: number;
  upspeed: number;
  eta: number | null;
  state: TorrentState;
  stateRaw: string;
  category: string;
  savePath: string;
  numSeeds: number;
  numLeechs: number;
  ratio: number;
  addedOn: number;
  completionOn: number;
}

export interface TransferSummary {
  dlSpeed: number;
  upSpeed: number;
  dlData: number;
  upData: number;
  connectionStatus: string;
}

export interface Category {
  name: string;
  save_path: string;
}

export interface AddOptions {
  savePath?: string | null;
  category?: string | null;
  stopped?: boolean | null;
  preferMagnet?: boolean;
}

export interface AppStatus {
  qbitRunning: boolean;
  qbitVersion: string | null;
  apiRunning: boolean;
  loggedIn: boolean;
  username: string | null;
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
}

export interface QbitConfig {
  binary_path: string;
  port: number;
  autostart: boolean;
}

export interface ApiConfig {
  enabled: boolean;
  host: string;
  port: number;
  token: string;
}

export interface DownloadsConfig {
  default_save_path: string;
  add_stopped: boolean;
}

export interface AppConfig {
  rutracker: RutrackerConfig;
  qbittorrent: QbitConfig;
  api: ApiConfig;
  downloads: DownloadsConfig;
}

export interface CommandError {
  code: string;
  message: string;
}

export interface DownloadsUpdate {
  items: DownloadItem[];
  transfer: TransferSummary;
}
