//! Типизированные модели данных трекера.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Статус модерации раздачи.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Проверено модератором.
    Approved,
    /// Не проверено.
    NotApproved,
    /// Недооформлено.
    NeedEdit,
    /// Повтор.
    Duplicate,
    /// Поглощено другой раздачей.
    Consumed,
    /// Закрыто.
    Closed,
    /// Сомнительно.
    Doubtful,
    /// Временная.
    Temporary,
    /// Не удалось распознать статус.
    Unknown,
}

impl ApprovalStatus {
    /// Определяет статус по классам иконки в столбце статуса
    /// (например, `tor-icon tor-approved`).
    pub fn from_icon_classes(classes: &str) -> Self {
        let has = |needle: &str| classes.split_whitespace().any(|c| c == needle);
        if has("tor-approved") {
            Self::Approved
        } else if has("tor-not-approved") {
            Self::NotApproved
        } else if has("tor-need-edit") {
            Self::NeedEdit
        } else if has("tor-dup") {
            Self::Duplicate
        } else if has("tor-consumed") {
            Self::Consumed
        } else if has("tor-closed") {
            Self::Closed
        } else if has("tor-dl-x") || has("tor-doubtful") {
            Self::Doubtful
        } else if has("tor-tmp") {
            Self::Temporary
        } else {
            Self::Unknown
        }
    }
}

/// Ссылка на форум (раздел) трекера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ForumRef {
    pub id: i64,
    pub name: String,
}

/// Строка результатов поиска.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResult {
    pub topic_id: u64,
    pub title: String,
    pub forum: Option<ForumRef>,
    pub author: Option<String>,
    /// Размер в байтах (из скрытого поля таблицы).
    pub size_bytes: u64,
    pub seeders: u64,
    pub leechers: u64,
    pub downloads: u64,
    /// Unix-время регистрации раздачи.
    pub added_unix: i64,
    pub approval: ApprovalStatus,
}

/// Страница результатов поиска.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchPage {
    pub items: Vec<SearchResult>,
    /// Всего найдено на сервере (максимум у rutracker — 500).
    pub total_found: u64,
    /// Смещение этой страницы.
    pub offset: u32,
    /// Токен продолжения для следующих страниц.
    pub search_id: Option<String>,
}

impl SearchPage {
    /// Есть ли ещё страницы результатов.
    pub fn has_more(&self) -> bool {
        u64::from(self.offset) + (self.items.len() as u64) < self.total_found
    }
}

/// Поле сортировки поиска (серверная сортировка rutracker).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Registered,
    Title,
    Downloads,
    Size,
    #[default]
    Seeders,
    Leechers,
}

impl SortField {
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Registered => 1,
            Self::Title => 2,
            Self::Downloads => 4,
            Self::Size => 7,
            Self::Seeders => 10,
            Self::Leechers => 11,
        }
    }
}

/// Направление сортировки.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

impl SortOrder {
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Asc => 1,
            Self::Desc => 2,
        }
    }
}

/// Параметры поискового запроса.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SearchRequest {
    /// Текст запроса.
    pub query: String,
    /// Идентификаторы форумов (пусто — все).
    pub forums: Vec<i64>,
    /// Фильтр по автору раздачи.
    pub author: Option<String>,
    pub sort: SortField,
    pub order: SortOrder,
    /// Смещение (кратно 50).
    pub offset: u32,
    /// Токен продолжения, полученный от первой страницы.
    pub search_id: Option<String>,
}

/// Вызов капчи на форме логина.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CaptchaChallenge {
    /// Значение скрытого поля `cap_sid`.
    pub sid: String,
    /// Имя текстового поля с кодом (вида `cap_code_XXXX`).
    pub code_field: String,
    /// Абсолютный URL картинки с капчей.
    pub img_url: String,
}

/// Ответ пользователя на капчу.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CaptchaAnswer {
    pub sid: String,
    pub code_field: String,
    pub value: String,
}

/// Состояние сессии на трекере.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    pub logged_in: bool,
    pub username: Option<String>,
}

/// Сводная статистика торрента на странице раздачи.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TorrentStats {
    pub size_bytes: Option<u64>,
    pub seeders: Option<u64>,
    pub leechers: Option<u64>,
    /// Сколько раз скачали.
    pub completed: Option<u64>,
    /// Дата регистрации торрента (как текст страницы).
    pub registered: Option<String>,
}

/// Страница раздачи.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopicPage {
    pub id: u64,
    pub title: String,
    /// Путь по разделам (хлебные крошки).
    pub forum_path: Vec<ForumRef>,
    /// Magnet-ссылка (доступна и без логина).
    pub magnet: Option<String>,
    /// Присутствует ли ссылка на `.torrent` (требует логина).
    pub has_torrent_file: bool,
    pub stats: TorrentStats,
    /// Содержимое первого поста в виде блоков.
    pub body: Vec<ContentBlock>,
}

/// Блок содержимого раздачи.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Image {
        src: String,
    },
    Spoiler {
        title: String,
        #[schema(no_recursion)]
        blocks: Vec<ContentBlock>,
    },
    Quote {
        author: Option<String>,
        #[schema(no_recursion)]
        blocks: Vec<ContentBlock>,
    },
    Code {
        text: String,
    },
    List {
        ordered: bool,
        #[schema(no_recursion)]
        items: Vec<Vec<ContentBlock>>,
    },
    Hr,
}

/// Строчный элемент внутри параграфа.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        bold: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        italic: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        underline: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        strike: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    Link {
        href: String,
        text: String,
        /// Идентификатор темы, если ссылка ведёт на раздачу rutracker.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        topic_id: Option<u64>,
    },
    /// Изображение в потоке текста (флаг, иконка, скриншот) — сохраняет исходный
    /// размер и не разрывает строку.
    Image {
        src: String,
    },
    Break,
}

/// Группа форумов в дереве категорий.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForumGroup {
    pub title: String,
    pub forums: Vec<ForumEntry>,
}

/// Форум в дереве категорий.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForumEntry {
    pub id: i64,
    pub name: String,
    /// Уровень вложенности (0 — корневой форум группы).
    pub depth: u8,
}

/// Скачанный `.torrent`-файл.
#[derive(Debug, Clone)]
pub struct TorrentFile {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
}
