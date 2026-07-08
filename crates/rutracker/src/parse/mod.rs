//! Разбор HTML-страниц rutracker в типизированные модели.
//!
//! Парсеры устойчивы к мелким изменениям разметки: отсутствующие поля
//! деградируют в значения по умолчанию, а не в ошибку всей страницы.

pub mod categories;
pub mod common;
pub mod login;
pub mod search;
pub mod text;
pub mod topic;
