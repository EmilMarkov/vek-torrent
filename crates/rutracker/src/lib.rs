//! Клиент rutracker: сессия с куками, логин (включая капчу), зеркала и прокси,
//! поиск по трекеру, разбор страницы раздачи в структурную блочную модель,
//! скачивание `.torrent`-файлов.
//!
//! Наполняется на этапе 2.

/// Версия крейта (проверка сборки скелета).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!super::VERSION.is_empty());
    }
}
