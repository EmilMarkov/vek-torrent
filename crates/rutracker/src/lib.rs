//! Клиент rutracker: сессия с куками, логин (включая капчу), зеркала и прокси,
//! поиск по трекеру, разбор страницы раздачи в структурную блочную модель,
//! скачивание `.torrent`-файлов.
//!
//! Особенности трекера, учтённые здесь:
//! - кодировка windows-1251 (и в ответах, и в телах POST-запросов);
//! - пагинация поиска через `search_id` + `start`;
//! - капча на форме логина после нескольких неудачных попыток;
//! - разметка, которую нельзя вставлять в webview как есть, — контент раздачи
//!   разбирается в типизированные блоки ([`models::ContentBlock`]).

pub mod client;
pub mod encoding;
pub mod error;
pub mod models;
pub mod parse;

pub use client::{Client, ClientBuilder, DEFAULT_MIRRORS};
pub use error::Error;

/// Результат операций крейта.
pub type Result<T> = std::result::Result<T, Error>;
