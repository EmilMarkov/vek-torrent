//! Клиент rutracker: сессия с куками, логин (включая капчу), зеркала и прокси,
//! поиск по трекеру, разбор страницы раздачи, скачивание `.torrent`-файлов.
//!
//! Особенности трекера, учтённые здесь:
//! - кодировка windows-1251 (и в ответах, и в телах POST-запросов);
//! - пагинация поиска через `search_id` + `start`;
//! - капча на форме логина после нескольких неудачных попыток;
//! - разметку поста нельзя вставлять в webview как есть — она проходит
//!   строгую санитизацию ([`parse::sanitize`]) с сохранением родных классов
//!   rutracker, которые фронтенд стилизует под тёмную тему.

pub mod client;
pub mod encoding;
pub mod error;
pub mod models;
pub mod parse;

pub use client::{Client, ClientBuilder, DEFAULT_MIRRORS};
pub use error::Error;

/// Результат операций крейта.
pub type Result<T> = std::result::Result<T, Error>;
